mod resp;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::Instant;

static DB: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug, PartialEq)]
enum Value {
    String(Vec<u8>),
    List(Vec<Vec<u8>>),
}

#[derive(Clone, Debug, PartialEq)]
struct Entry {
    value: Value,
    expiry: Option<Instant>,
}

impl Entry {
    fn new(value: Value, expiry: Option<Instant>) -> Self {
        Self { value, expiry }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 后台每秒扫描过期 key
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut db = DB.lock().unwrap();
            db.retain(|_, entry| !entry.expiry.is_some_and(|exp| Instant::now() >= exp));
        }
    });

    let listener = TcpListener::bind("127.0.0.1:6379")
        .await
        .context("failed to bind to 127.0.0.1:6379")?;

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
    let mut read_buf = [0u8; 512];
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

        // 检测模式：pending 为空时根据首字节判断
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
        dispatch_command(&cmd, &args, stream).await?;
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

                if let resp::RespType::Array(Some(items)) = &frame {
                    let cmd = items.first().and_then(|v| {
                        if let resp::RespType::BulkString(Some(bytes)) = v {
                            Some(bytes.as_slice())
                        } else {
                            None
                        }
                    });

                    if let Some(cmd) = cmd {
                        let cmd_str = String::from_utf8_lossy(cmd).to_uppercase();
                        let args: Vec<String> = items[1..]
                            .iter()
                            .filter_map(|v| {
                                if let resp::RespType::BulkString(Some(bytes)) = v {
                                    Some(String::from_utf8_lossy(bytes).to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        dispatch_command(&cmd_str, &args, stream).await?;
                    }
                }
            }
            Err(resp::DecodeError::Incomplete) => break,
            Err(resp::DecodeError::Invalid(e)) => {
                eprintln!("decode error: {}", e);
                let err = resp::RespType::Error(format!("ERR protocol error: {}", e));
                stream
                    .write_all(&err.serialize())
                    .await
                    .context("failed to write protocol error response")?;
                return Ok(());
            }
        }
    }
    Ok(())
}

async fn dispatch_command(
    cmd: &str,
    args: &[String],
    stream: &mut tokio::net::TcpStream,
) -> anyhow::Result<()> {
    match cmd {
        "PING" => {
            let response = resp::RespType::SimpleString("PONG".to_string());
            stream
                .write_all(&response.serialize())
                .await
                .context("failed to write PONG response")?;
        }
        "ECHO" => {
            if let Some(arg) = args.first() {
                let response = resp::RespType::BulkString(Some(arg.as_bytes().to_vec()));
                stream
                    .write_all(&response.serialize())
                    .await
                    .context("failed to write ECHO response")?;
            } else {
                let err = resp::RespType::Error(
                    "ERR wrong number of arguments for 'echo' command".to_string(),
                );
                stream
                    .write_all(&err.serialize())
                    .await
                    .context("failed to write error response")?;
            }
        }
        "SET" => {
            if args.len() == 2 {
                {
                    let mut db = DB.lock().unwrap();
                    db.insert(
                        args[0].clone(),
                        Entry::new(Value::String(args[1].as_bytes().to_vec()), None),
                    );
                }
                let response = resp::RespType::SimpleString("OK".to_string());
                stream.write_all(&response.serialize()).await?;
            } else if args.len() == 4 {
                {
                    let mut db = DB.lock().unwrap();
                    db.insert(
                        args[0].clone(),
                        Entry::new(
                            Value::String(args[1].as_bytes().to_vec()),
                            Some(
                                Instant::now()
                                    + if args[2] == "PX" {
                                        Duration::from_millis(args[3].parse::<u64>()?)
                                    } else if args[2] == "EX" {
                                        Duration::from_secs(args[3].parse::<u64>()?)
                                    } else {
                                        Duration::ZERO
                                    },
                            ),
                        ),
                    );
                }
                let response = resp::RespType::SimpleString("OK".to_string());
                stream.write_all(&response.serialize()).await?;
            } else {
                let err = resp::RespType::Error(
                    "ERR wrong number of arguments for 'set' command".to_string(),
                );
                stream.write_all(&err.serialize()).await?;
            }
        }
        "GET" => {
            if let Some(key) = args.first() {
                let value = {
                    let mut db = DB.lock().unwrap();
                    match db.get(key) {
                        Some(entry) => {
                            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                                db.remove(key);
                                None
                            } else {
                                Some(entry.value.clone())
                            }
                        }
                        None => None,
                    }
                };
                let response = match value {
                    Some(v) => resp::RespType::BulkString(match v {
                        Value::String(u) => Some(u),
                        _ => unreachable!(),
                    }),
                    None => resp::RespType::BulkString(None),
                };
                stream.write_all(&response.serialize()).await?;
            } else {
                let err = resp::RespType::Error(
                    "ERR wrong number of arguments for 'get' command".to_string(),
                );
                stream.write_all(&err.serialize()).await?;
            }
        }
        "RPUSH" => {
            if args.len() == 2 {
                let (key, value) = (args[0].clone(), args[1].as_bytes().to_vec());
                let response = {
                    let mut db = DB.lock().unwrap();
                    match db.get_mut(&key) {
                        Some(entry) => {
                            if let Value::List(ref mut list) = entry.value {
                                list.push(value);
                                resp::RespType::Integer(list.len() as i64)
                            } else {
                                resp::RespType::Error(
                                    "WRONGTYPE Operation against a key holding the wrong kind of value"
                                        .to_string(),
                                )
                            }
                        }
                        None => {
                            db.insert(key, Entry::new(Value::List(vec![value]), None));
                            resp::RespType::Integer(1)
                        }
                    }
                }; // lock dropped here
                stream.write_all(&response.serialize()).await?;
            } else {
                let err = resp::RespType::Error(
                    "ERR wrong number of arguments for 'rpush' command".to_string(),
                );
                stream.write_all(&err.serialize()).await?;
            }
        }
        _ => {
            let err = resp::RespType::Error("ERR unknown command".to_string());
            stream
                .write_all(&err.serialize())
                .await
                .context("failed to write error response")?;
        }
    }
    Ok(())
}
