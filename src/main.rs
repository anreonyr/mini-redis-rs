use codecrafters_redis::{cmd, db, inline, resp};
use std::time::Duration;
use tokio::time::Instant;

use anyhow::Context;
use inline::{apply_backspace, find_line, parse_quoted_args, strip_iac};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .context("failed to bind to 127.0.0.1:6379")?;

    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            db::with_db(|db| {
                db.retain(|_, entry| !entry.expiry.is_some_and(|exp| Instant::now() >= exp));
            });
        }
    });

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .context("failed to accept connection")?;
        println!("accepted new connection");

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("connection error: {:#}", e);
            }
        });
    }
}

async fn handle_connection(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let decoder = resp::Decoder::new();
    let mut read_buf = [0u8; 8192];
    let mut pending = Vec::new();
    let mut inline_mode = false;

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
            process_inline(&mut pending, &mut stream).await?;
        } else {
            process_resp(&decoder, &mut pending, &mut stream).await?;
        }
    }
}

async fn process_inline(
    pending: &mut Vec<u8>,
    stream: &mut tokio::net::TcpStream,
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
                let response = cmd::dispatch_command(cmd::ParsedCmd::parse(&cmd, cmd_args)).await;
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
) -> anyhow::Result<()> {
    loop {
        match decoder.decode(pending) {
            Ok((frame, consumed)) => {
                pending.drain(..consumed);
                println!("received: {}", frame);

                if let Some(cmd) = cmd::parse_command(&frame) {
                    let response = cmd::dispatch_command(cmd).await;
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
