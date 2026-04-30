mod cmd;
mod db;
mod resp;

use std::time::Duration;
use tokio::time::Instant;

use anyhow::Context;
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
    while let Some(pos) = pending.windows(2).position(|w| w == b"\r\n") {
        let line = String::from_utf8_lossy(&pending[..pos]).trim().to_string();
        pending.drain(..pos + 2);

        if line.is_empty() {
            continue;
        }

        println!("received inline: {}", line);

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_uppercase();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
        let response = cmd::dispatch_command(&cmd, &args);
        send_response(stream, &response).await?;
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

                if let Some((cmd, args)) = cmd::parse_command(&frame) {
                    let response = cmd::dispatch_command(&cmd, &args);
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
