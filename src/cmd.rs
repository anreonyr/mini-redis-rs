use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::Instant;

use crate::db::{Entry, Value, with_db};
use crate::resp;

#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("ERR wrong number of arguments for '{0}' command")]
    WrongArgCount(String),
    #[error("ERR value is not an integer or out of range")]
    InvalidInteger,
    #[error("ERR syntax error")]
    SyntaxError,
}

fn wrong_arg_count(cmd: &str) -> CmdError {
    CmdError::WrongArgCount(cmd.to_string())
}

pub fn parse_command(frame: &resp::RespType) -> Option<(String, Vec<String>)> {
    if let resp::RespType::Array(Some(items)) = frame {
        let cmd = items.first().and_then(|v| {
            if let resp::RespType::BulkString(Some(bytes)) = v {
                Some(String::from_utf8_lossy(bytes).to_uppercase())
            } else {
                None
            }
        });
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
        cmd.map(|c| (c, args))
    } else {
        None
    }
}

pub fn dispatch_command(cmd: &str, args: &[String]) -> resp::RespType {
    (match cmd {
        "PING" => handle_ping(args),
        "ECHO" => handle_echo(args),
        "SET" => handle_set(args),
        "GET" => handle_get(args),
        "RPUSH" => handle_rpush(args),
        "LPUSH" => handle_lpush(args),
        "LRANGE" => handle_lrange(args),
        "LLEN" => handle_llen(args),
        _ => return resp::RespType::Error("ERR unknown command".to_string()),
    })
    .unwrap_or_else(|e| resp::RespType::Error(e.to_string()))
}

fn handle_ping(_args: &[String]) -> anyhow::Result<resp::RespType> {
    Ok(resp::RespType::SimpleString("PONG".to_string()))
}

fn handle_echo(args: &[String]) -> anyhow::Result<resp::RespType> {
    match args.first() {
        Some(arg) => Ok(resp::RespType::BulkString(Some(arg.as_bytes().to_vec()))),
        None => Err(wrong_arg_count("echo").into()),
    }
}

fn handle_set(args: &[String]) -> anyhow::Result<resp::RespType> {
    match args.len() {
        2 => {
            with_db(|db| {
                db.insert(
                    args[0].clone(),
                    Entry::new(Value::String(args[1].as_bytes().to_vec()), None),
                );
            });
            Ok(resp::RespType::SimpleString("OK".to_string()))
        }
        4 => {
            let dur = match args[2].as_str() {
                "PX" => args[3]
                    .parse::<u64>()
                    .map(Duration::from_millis)
                    .map_err(|_| CmdError::InvalidInteger)?,
                "EX" => args[3]
                    .parse::<u64>()
                    .map(Duration::from_secs)
                    .map_err(|_| CmdError::InvalidInteger)?,
                _ => anyhow::bail!(CmdError::SyntaxError),
            };
            with_db(|db| {
                db.insert(
                    args[0].clone(),
                    Entry::new(
                        Value::String(args[1].as_bytes().to_vec()),
                        Some(Instant::now() + dur),
                    ),
                );
            });
            Ok(resp::RespType::SimpleString("OK".to_string()))
        }
        _ => Err(wrong_arg_count("set").into()),
    }
}

fn handle_get(args: &[String]) -> anyhow::Result<resp::RespType> {
    match args.first() {
        Some(key) => Ok(with_db(|db| match db.get(key) {
            Some(entry) => {
                if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                    db.remove(key);
                    resp::RespType::BulkString(None)
                } else {
                    match entry.value.clone() {
                        Value::String(v) => resp::RespType::BulkString(Some(v)),
                        Value::List(_) => resp::RespType::Error(
                            "WRONGTYPE Operation against a key holding the wrong kind of value"
                                .to_string(),
                        ),
                    }
                }
            }
            None => resp::RespType::BulkString(None),
        })),
        None => Err(wrong_arg_count("get").into()),
    }
}

fn handle_rpush(args: &[String]) -> anyhow::Result<resp::RespType> {
    if args.len() >= 2 {
        let values: VecDeque<Vec<u8>> = args[1..].iter().map(|v| v.as_bytes().to_vec()).collect();
        let key = args[0].clone();
        Ok(with_db(|db| match db.get_mut(&key) {
            Some(entry) => {
                if let Value::List(ref mut list) = entry.value {
                    list.extend(values);
                    resp::RespType::Integer(list.len() as i64)
                } else {
                    resp::RespType::Error(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    )
                }
            }
            None => {
                let len = values.len();
                db.insert(key, Entry::new(Value::List(values), None));
                resp::RespType::Integer(len as i64)
            }
        }))
    } else {
        Err(wrong_arg_count("rpush").into())
    }
}

fn handle_lpush(args: &[String]) -> anyhow::Result<resp::RespType> {
    if args.len() >= 2 {
        let values: VecDeque<Vec<u8>> = args[1..].iter().map(|v| v.as_bytes().to_vec()).collect();
        let key = &args[0];
        Ok(with_db(|db| match db.get_mut(key) {
            Some(entry) => {
                if let Value::List(ref mut list) = entry.value {
                    for v in values {
                        list.push_front(v);
                    }
                    resp::RespType::Integer(list.len() as i64)
                } else {
                    resp::RespType::Error(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                            .to_string(),
                    )
                }
            }
            None => {
                let len = values.len();
                db.insert(key.to_owned(), Entry::new(Value::List(values), None));
                resp::RespType::Integer(len as i64)
            }
        }))
    } else {
        Err(wrong_arg_count("lpush").into())
    }
}

fn handle_lrange(args: &[String]) -> anyhow::Result<resp::RespType> {
    if args.len() == 3 {
        let key = &args[0];
        let start = args[1]
            .parse::<i64>()
            .map_err(|_| CmdError::InvalidInteger)?;
        let stop = args[2]
            .parse::<i64>()
            .map_err(|_| CmdError::InvalidInteger)?;

        Ok(with_db(|db| match db.get(key) {
            Some(entry) => match entry.value.clone() {
                Value::List(list) => {
                    let len = list.len() as i64;
                    if len == 0 {
                        return resp::RespType::Array(Some(vec![]));
                    }

                    // Convert negative indices to positive
                    let mut l = if start < 0 { len + start } else { start };
                    let mut r = if stop < 0 { len + stop } else { stop };

                    // Clamp to valid range
                    if l < 0 {
                        l = 0;
                    }
                    if r >= len {
                        r = len - 1;
                    }

                    if l > r {
                        resp::RespType::Array(Some(vec![]))
                    } else {
                        let items: Vec<resp::RespType> = list
                            .range(l as usize..=r as usize)
                            .map(|v| resp::RespType::BulkString(Some(v.clone())))
                            .collect();
                        resp::RespType::Array(Some(items))
                    }
                }
                _ => resp::RespType::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                ),
            },
            None => resp::RespType::Array(Some(vec![])),
        }))
    } else {
        Err(wrong_arg_count("lrange").into())
    }
}

fn handle_llen(args: &[String]) -> anyhow::Result<resp::RespType> {
    match args.first() {
        Some(key) => Ok(with_db(|db| match db.get(key) {
            Some(v) => {
                if let Value::List(u) = &v.value {
                    resp::RespType::Integer(u.len() as i64)
                } else {
                    resp::RespType::Integer(0)
                }
            }
            None => resp::RespType::Integer(0),
        })),
        None => Err(wrong_arg_count("llen").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resp;

    #[test]
    fn test_handle_ping() {
        assert_eq!(
            handle_ping(&[]).unwrap(),
            resp::RespType::SimpleString("PONG".to_string())
        );
    }

    #[test]
    fn test_handle_echo() {
        assert_eq!(
            handle_echo(&["hello".to_string()]).unwrap(),
            resp::RespType::BulkString(Some(b"hello".to_vec()))
        );
    }

    #[test]
    fn test_handle_echo_no_args() {
        assert_eq!(
            handle_echo(&[]).unwrap_err().to_string(),
            "ERR wrong number of arguments for 'echo' command"
        );
    }

    #[test]
    fn test_wrong_arg_count() {
        assert_eq!(
            wrong_arg_count("test").to_string(),
            "ERR wrong number of arguments for 'test' command"
        );
    }

    #[test]
    fn test_parse_command() {
        let frame = resp::RespType::Array(Some(vec![
            resp::RespType::BulkString(Some(b"GET".to_vec())),
            resp::RespType::BulkString(Some(b"key".to_vec())),
        ]));
        let (cmd, args) = parse_command(&frame).unwrap();
        assert_eq!(cmd, "GET");
        assert_eq!(args, vec!["key".to_string()]);
    }

    #[test]
    fn test_parse_command_not_array() {
        let frame = resp::RespType::SimpleString("OK".to_string());
        assert!(parse_command(&frame).is_none());
    }

    #[test]
    fn test_dispatch_command_ping() {
        assert_eq!(
            dispatch_command("PING", &[]),
            resp::RespType::SimpleString("PONG".to_string())
        );
    }

    #[test]
    fn test_dispatch_command_unknown() {
        assert_eq!(
            dispatch_command("UNKNOWN", &[]),
            resp::RespType::Error("ERR unknown command".to_string())
        );
    }

    #[test]
    fn test_dispatch_command_echo() {
        assert_eq!(
            dispatch_command("ECHO", &["foo".to_string()]),
            resp::RespType::BulkString(Some(b"foo".to_vec()))
        );
    }

    #[test]
    fn test_set_get_roundtrip() {
        let result = dispatch_command("SET", &["rt1".to_string(), "val1".to_string()]);
        assert_eq!(result, resp::RespType::SimpleString("OK".to_string()));

        let result = dispatch_command("GET", &["rt1".to_string()]);
        assert_eq!(result, resp::RespType::BulkString(Some(b"val1".to_vec())));
    }

    #[test]
    fn test_get_nonexistent_key() {
        let result = dispatch_command("GET", &["nonexistent".to_string()]);
        assert_eq!(result, resp::RespType::BulkString(None));
    }

    #[test]
    fn test_get_no_args() {
        let result = dispatch_command("GET", &[]);
        assert_eq!(
            result,
            resp::RespType::Error("ERR wrong number of arguments for 'get' command".to_string())
        );
    }

    #[test]
    fn test_handle_set_with_invalid_expiry() {
        let result = dispatch_command(
            "SET",
            &[
                "k".to_string(),
                "v".to_string(),
                "EX".to_string(),
                "not_a_number".to_string(),
            ],
        );
        assert_eq!(
            result,
            resp::RespType::Error("ERR value is not an integer or out of range".to_string())
        );
    }

    #[test]
    fn test_handle_set_with_unknown_flag() {
        let result = dispatch_command(
            "SET",
            &[
                "k".to_string(),
                "v".to_string(),
                "XX".to_string(),
                "100".to_string(),
            ],
        );
        assert_eq!(
            result,
            resp::RespType::Error("ERR syntax error".to_string())
        );
    }
}
