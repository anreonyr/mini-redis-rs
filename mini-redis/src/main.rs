use mini_redis::{cmd, server::{config, registry, shutdown}, storage::{db, persist}, protocol::{inline, resp}};
use mini_redis::storage::db::DB_INDEX;
use std::time::Duration;
use tokio::time::Instant;

use anyhow::Context;
use inline::{apply_backspace, find_line, parse_quoted_args, strip_iac};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinSet;

use mini_redis::server::pubsub::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    registry::init();

    // Parse --requirepass CLI arg
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--requirepass") {
        if let Some(password) = args.get(pos + 1) {
            config::set_requirepass_from_cli(password.clone());
        }
    }

    // Auto-load persistence file on startup
    let path = config::with_config(|cfg| cfg.db_path());
    if persist::file_exists(&path) {
        DB_INDEX.scope(std::cell::Cell::new(0), async {
            match persist::load(&path) {
                Ok(n) => println!("Loaded {} keys from {}", n, path),
                Err(e) => eprintln!("Failed to load persistence file: {}", e),
            }
        }).await;
    }

    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .context("failed to bind to 127.0.0.1:6379")?;

    let eviction = tokio::spawn(async {
        DB_INDEX.scope(std::cell::Cell::new(0), async {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if shutdown::is_requested() {
                    break;
                }
                db::with_db(|db| {
                    db.retain(|_, entry| !entry.expiry.is_some_and(|exp| Instant::now() >= exp));
                });
            }
        }).await;
    });

    let mut connections = JoinSet::new();

    let result = tokio::select! {
        r = accept_loop(&listener, &mut connections) => r,
        _ = signal::ctrl_c() => {
            println!("\nCtrl+C received, saving data...");
            shutdown::request();
            let path = config::with_config(|cfg| cfg.db_path());
            DB_INDEX.scope(std::cell::Cell::new(0), async {
                if let Err(e) = persist::save(&path).await {
                    eprintln!("Failed to save data: {}", e);
                } else {
                    println!("Data saved to {}", path);
                }
            }).await;
            println!("Shutting down...");
            Ok(())
        }
    };

    // ── Graceful shutdown ──
    println!("Stopping eviction task...");
    eviction.abort();

    println!(
        "Waiting for {} active connection(s) to finish...",
        connections.len()
    );
    while let Some(r) = connections.join_next().await {
        if let Err(e) = r {
            eprintln!("connection task panicked: {e}");
        }
    }

    println!("Server stopped.");
    result
}

async fn accept_loop(
    listener: &TcpListener,
    connections: &mut JoinSet<()>,
) -> anyhow::Result<()> {
    loop {
        if shutdown::is_requested() {
            println!("shutdown requested, stop accepting new connections");
            return Ok(());
        }

        let (stream, _) = listener
            .accept()
            .await
            .context("failed to accept connection")?;
        println!("accepted new connection");

        connections.spawn(async move {
            // Each connection runs in its own task-local DB scope (default DB 0).
            // SELECT command calls db::set_current_db() to change within this scope.
            DB_INDEX.scope(std::cell::Cell::new(0), async move {
                if let Err(e) = handle_connection(stream).await {
                    eprintln!("connection error: {:#}", e);
                }
            }).await;
        });
    }
}

/// Push incoming pub/sub messages to the client until the channel is closed.
async fn push_pubsub_messages(
    stream: &mut tokio::net::TcpStream,
    mut rx: UnboundedReceiver<Message>,
) -> anyhow::Result<()> {
    let mut read_buf = [0u8; 64]; // small buffer, we only check for disconnect
    loop {
        tokio::select! {
            // Check if client disconnected (socket read returns 0)
            result = stream.read(&mut read_buf) => {
                let n = result.context("pubsub read error")?;
                if n == 0 { return Ok(()); }
                // For now, ignore incoming commands in subscription mode
                // A full implementation would handle SUBSCRIBE/UNSUBSCRIBE
            }
            msg = rx.recv() => {
                match msg {
                    Some(msg) => {
                        let response = resp::RespType::Array(Some(vec![
                            resp::RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"message"))),
                            resp::RespType::BulkString(Some(bytes::Bytes::copy_from_slice(msg.channel.as_bytes()))),
                            resp::RespType::BulkString(Some(bytes::Bytes::copy_from_slice(msg.payload.as_bytes()))),
                        ]));
                        stream.write_all(&response.serialize()).await
                            .context("failed to write pubsub message")?;
                    }
                    None => {
                        // Channel closed, subscription ended
                        return Ok(());
                    }
                }
            }
        }
    }
}

async fn handle_connection(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let decoder = resp::Decoder::new();
    let mut read_buf = [0u8; 8192];
    let mut pending = Vec::new();
    let mut inline_mode = false;
    let mut state = cmd::ConnectionState::new();

    loop {
        let n = stream
            .read(&mut read_buf)
            .await
            .context("failed to read from stream")?;

        if n == 0 {
            return Ok(());
        }

        if pending.is_empty() {
            inline_mode = !matches!(read_buf[0], b'+' | b'-' | b':' | b'$' | b'*');
        }

        pending.extend_from_slice(&read_buf[..n]);

        // Prevent unbounded pending buffer growth
        const MAX_PENDING: usize = 1024 * 1024;
        if pending.len() > MAX_PENDING {
            pending.clear();
            let err = resp::RespType::Error("ERR inline buffer too large".to_string());
            send_response(&mut stream, &err).await?;
            continue;
        }

        if inline_mode {
            process_inline(&mut pending, &mut stream, &mut state).await?;
        } else {
            process_resp(&decoder, &mut pending, &mut stream, &mut state).await?;
        }

        if state.is_subscribed() {
            // Subscription mode: take the rx and enter push loop
            let sub = state.subscription.take();
            if let Some(sub_state) = sub {
                push_pubsub_messages(&mut stream, sub_state.rx).await?;
            }
            // After push loop returns (client disconnected or channel closed),
            // continue the outer loop
        }

        if state.quit {
            return Ok(());
        }
    }
}

async fn process_inline(
    pending: &mut Vec<u8>,
    stream: &mut tokio::net::TcpStream,
    state: &mut cmd::ConnectionState,
) -> anyhow::Result<()> {
    // Clean the buffer before processing lines
    strip_iac(pending);
    apply_backspace(pending);

    // Process all complete lines
    while let Some((pos, delim_len)) = find_line(pending) {
        let line = String::from_utf8_lossy(&pending[..pos]).trim().to_string();
        pending.drain(..pos + delim_len);

        if line.is_empty() {
            continue;
        }

        println!("received inline: {}", line);

        match parse_quoted_args(&line) {
            Ok(args) if args.is_empty() => continue,
            Ok(args) => {
                let cmd = args[0].to_uppercase();
                let cmd_args: Vec<String> = args[1..].to_vec();
                let response = cmd::dispatch_command(cmd::ParsedCmd::parse(&cmd, cmd_args), state).await;
                send_response(stream, &response).await?;
            }
            Err(e) => {
                let err = resp::RespType::Error(e);
                send_response(stream, &err).await?;
            }
        }
    }
    Ok(())
}

async fn process_resp(
    decoder: &resp::Decoder,
    pending: &mut Vec<u8>,
    stream: &mut tokio::net::TcpStream,
    state: &mut cmd::ConnectionState,
) -> anyhow::Result<()> {
    loop {
        match decoder.decode(pending) {
            Ok((frame, consumed)) => {
                pending.drain(..consumed);
                println!("received: {}", frame);

                if let Some(cmd) = cmd::parse_command(&frame) {
                    let response = cmd::dispatch_command(cmd, state).await;
                    send_response(stream, &response).await?;
                }
            }
            Err(resp::DecodeError::Incomplete) => break,
            Err(resp::DecodeError::Invalid(e)) => {
                eprintln!("decode error: {}", e);
                let err = resp::RespType::Error(format!("ERR protocol error: {}", e));
                send_response(stream, &err).await?;
                return Ok(());
            }
        }
    }
    Ok(())
}

async fn send_response(
    stream: &mut tokio::net::TcpStream,
    response: &resp::RespType,
) -> anyhow::Result<()> {
    stream
        .write_all(&response.serialize())
        .await
        .context("failed to write response")
}
