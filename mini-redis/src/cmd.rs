use bytes::Bytes;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::Instant;

use crate::blocking;
use crate::db::{Entry, StreamData, StreamEntry, Value, with_db};
use crate::registry;
use crate::resp;

/// All arguments have been parsed and validated at this point.
#[derive(Debug, PartialEq)]
pub enum ParsedCmd {
    Ping,
    Echo {
        message: String,
    },
    Set {
        key: String,
        value: String,
        expiry: Option<Duration>,
    },
    Get {
        key: String,
    },
    Rpush {
        key: String,
        values: Vec<String>,
    },
    Lpush {
        key: String,
        values: Vec<String>,
    },
    Lrange {
        key: String,
        start: i64,
        stop: i64,
    },
    Llen {
        key: String,
    },
    Lpop {
        key: String,
        count: Option<usize>,
    },
    Flushdb,
    Blpop {
        keys: Vec<String>,
        timeout: u64,
    },
    Command {
        subcommand: Option<String>,
        name: Option<String>,
    },
    // Streams
    Xadd {
        key: String,
        id: String,
        fields: Vec<String>,
    },
    Xrange {
        key: String,
        start: String,
        end: String,
        count: Option<u64>,
    },
    Xrevrange {
        key: String,
        end: String,
        start: String,
        count: Option<u64>,
    },
    Xlen {
        key: String,
    },
    Xtrim {
        key: String,
        strategy: String,
        threshold: u64,
        exact: bool,
    },
    Xdel {
        key: String,
        ids: Vec<String>,
    },
    Xread {
        count: Option<u64>,
        keys: Vec<String>,
        ids: Vec<String>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("ERR wrong number of arguments for '{0}' command")]
    WrongArgCount(String),
    #[error("ERR value is not an integer or out of range")]
    InvalidInteger,
    #[error("ERR syntax error")]
    SyntaxError,
    #[error("ERR unknown command")]
    UnknownCommand,
}

fn wrong_arg_count(cmd: &str) -> CmdError {
    CmdError::WrongArgCount(cmd.to_string())
}

fn wrong_type() -> resp::RespType {
    resp::RespType::Error(
        "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
    )
}

impl ParsedCmd {
    pub fn parse(cmd: &str, args: Vec<String>) -> Result<Self, CmdError> {
        Ok(match cmd {
            "PING" => ParsedCmd::Ping,
            "ECHO" => {
                let message = args
                    .into_iter()
                    .next()
                    .ok_or_else(|| wrong_arg_count("echo"))?;
                ParsedCmd::Echo { message }
            }
            "SET" => {
                if args.len() != 2 && args.len() != 4 {
                    return Err(wrong_arg_count("set"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let value = iter.next().unwrap();
                let expiry = match (iter.next(), iter.next()) {
                    (Some(flag), Some(val)) => Some(match flag.as_str() {
                        "PX" => Duration::from_millis(
                            val.parse().map_err(|_| CmdError::InvalidInteger)?,
                        ),
                        "EX" => {
                            Duration::from_secs(val.parse().map_err(|_| CmdError::InvalidInteger)?)
                        }
                        _ => return Err(CmdError::SyntaxError),
                    }),
                    (None, None) => None,
                    _ => return Err(wrong_arg_count("set")),
                };
                ParsedCmd::Set { key, value, expiry }
            }
            "GET" => {
                let key = args
                    .into_iter()
                    .next()
                    .ok_or_else(|| wrong_arg_count("get"))?;
                ParsedCmd::Get { key }
            }
            "RPUSH" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("rpush"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let values: Vec<String> = iter.collect();
                ParsedCmd::Rpush { key, values }
            }
            "LPUSH" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("lpush"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let values: Vec<String> = iter.collect();
                ParsedCmd::Lpush { key, values }
            }
            "LRANGE" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("lrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let start: i64 = iter
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(|_| CmdError::InvalidInteger)?;
                let stop: i64 = iter
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Lrange { key, start, stop }
            }
            "LLEN" => {
                let key = args
                    .into_iter()
                    .next()
                    .ok_or_else(|| wrong_arg_count("llen"))?;
                ParsedCmd::Llen { key }
            }
            "LPOP" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(wrong_arg_count("lpop"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let count = iter
                    .next()
                    .map(|s| s.parse::<usize>().map_err(|_| CmdError::InvalidInteger))
                    .transpose()?;
                ParsedCmd::Lpop { key, count }
            }
            "BLPOP" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("blpop"));
                }
                let timeout = args[args.len() - 1]
                    .parse()
                    .map_err(|_| CmdError::InvalidInteger)?;
                let mut a = args;
                a.pop();
                ParsedCmd::Blpop { keys: a, timeout }
            }
            "COMMAND" => {
                // COMMAND [INFO [name]] or just COMMAND
                let mut iter = args.into_iter();
                let subcommand = iter.next().map(|s| s.to_uppercase());
                let name = iter.next();
                ParsedCmd::Command { subcommand, name }
            }
            "FLUSHDB" => ParsedCmd::Flushdb,
            // Streams
            "XADD" => {
                if args.len() < 3 || args.len() % 2 != 0 {
                    return Err(wrong_arg_count("xadd"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let id = iter.next().unwrap();
                let fields: Vec<String> = iter.collect();
                ParsedCmd::Xadd { key, id, fields }
            }
            "XRANGE" => {
                if args.len() < 3 || args.len() > 5 {
                    return Err(wrong_arg_count("xrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let start = iter.next().unwrap();
                let end = iter.next().unwrap();
                let count = match (iter.next(), iter.next()) {
                    (Some(flag), Some(val)) if flag == "COUNT" => {
                        Some(val.parse::<u64>().map_err(|_| CmdError::InvalidInteger)?)
                    }
                    (None, None) => None,
                    _ => return Err(wrong_arg_count("xrange")),
                };
                ParsedCmd::Xrange { key, start, end, count }
            }
            "XREVRANGE" => {
                if args.len() < 3 || args.len() > 5 {
                    return Err(wrong_arg_count("xrevrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let end = iter.next().unwrap();
                let start = iter.next().unwrap();
                let count = match (iter.next(), iter.next()) {
                    (Some(flag), Some(val)) if flag == "COUNT" => {
                        Some(val.parse::<u64>().map_err(|_| CmdError::InvalidInteger)?)
                    }
                    (None, None) => None,
                    _ => return Err(wrong_arg_count("xrevrange")),
                };
                ParsedCmd::Xrevrange { key, end, start, count }
            }
            "XLEN" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("xlen"))?;
                ParsedCmd::Xlen { key }
            }
            "XTRIM" => {
                if args.len() < 3 || args.len() > 4 {
                    return Err(wrong_arg_count("xtrim"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let strategy = iter.next().ok_or_else(|| wrong_arg_count("xtrim"))?;
                if strategy.to_uppercase() != "MAXLEN" {
                    return Err(CmdError::SyntaxError);
                }
                let exact = match iter.next().as_deref() {
                    Some("~") => false,
                    Some(n) => {
                        let threshold = n.parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
                        return Ok(ParsedCmd::Xtrim { key, strategy, threshold, exact: true });
                    }
                    None => return Err(wrong_arg_count("xtrim")),
                };
                let threshold = iter.next().ok_or_else(|| wrong_arg_count("xtrim"))?
                    .parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Xtrim { key, strategy, threshold, exact }
            }
            "XDEL" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("xdel"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let ids: Vec<String> = iter.collect();
                ParsedCmd::Xdel { key, ids }
            }
            "XREAD" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("xread"));
                }
                // Check if first arg is COUNT
                let (count, remaining) = if args[0] == "COUNT" {
                    if args.len() < 3 {
                        return Err(wrong_arg_count("xread"));
                    }
                    let n = args[1].parse::<u64>()
                        .map_err(|_| CmdError::InvalidInteger)?;
                    (Some(n), &args[2..])
                } else {
                    (None, &args[..])
                };
                if remaining.is_empty() || remaining[0].to_uppercase() != "STREAMS" {
                    return Err(wrong_arg_count("xread"));
                }
                let all = &remaining[1..];
                if all.is_empty() || all.len() % 2 != 0 {
                    return Err(wrong_arg_count("xread"));
                }
                let mid = all.len() / 2;
                let keys = all[..mid].to_vec();
                let ids = all[mid..].to_vec();
                ParsedCmd::Xread { count, keys, ids }
            }
            _ => return Err(CmdError::UnknownCommand),
        })
    }
}

/// Parse a RESP frame into a parsed command.
/// Returns `None` if the frame is not a command array; `Some(Err(..))` for unknown commands
/// or invalid arguments.
pub fn parse_command(frame: &resp::RespType) -> Option<Result<ParsedCmd, CmdError>> {
    if let resp::RespType::Array(Some(items)) = frame {
        let cmd = items.first().and_then(|v| {
            if let resp::RespType::BulkString(Some(bytes)) = v {
                Some(String::from_utf8_lossy(bytes).to_uppercase())
            } else {
                None
            }
        })?;
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
        Some(ParsedCmd::parse(&cmd, args))
    } else {
        None
    }
}

pub async fn dispatch_command(cmd: Result<ParsedCmd, CmdError>) -> resp::RespType {
    let parsed = match cmd {
        Ok(c) => c,
        Err(e) => return resp::RespType::Error(e.to_string()),
    };
    match parsed {
        ParsedCmd::Ping => handle_ping(),
        ParsedCmd::Echo { message } => handle_echo(&message),
        ParsedCmd::Set { key, value, expiry } => handle_set(&key, &value, expiry),
        ParsedCmd::Get { key } => handle_get(&key),
        ParsedCmd::Rpush { key, values } => handle_rpush(&key, &values),
        ParsedCmd::Lpush { key, values } => handle_lpush(&key, &values),
        ParsedCmd::Lrange { key, start, stop } => handle_lrange(&key, start, stop),
        ParsedCmd::Llen { key } => handle_llen(&key),
        ParsedCmd::Lpop { key, count } => handle_lpop(&key, count),
        ParsedCmd::Blpop { keys, timeout } => handle_blpop(&keys, timeout).await,
        ParsedCmd::Command { subcommand, name } => handle_command(subcommand, name),
        ParsedCmd::Flushdb => handle_flushdb(),
        // Streams
        ParsedCmd::Xadd { key, id, fields } => handle_xadd(&key, &id, &fields),
        ParsedCmd::Xrange { key, start, end, count } => handle_xrange(&key, &start, &end, count),
        ParsedCmd::Xrevrange { key, end, start, count } => handle_xrevrange(&key, &end, &start, count),
        ParsedCmd::Xlen { key } => handle_xlen(&key),
        ParsedCmd::Xtrim { key, strategy, threshold, exact } => handle_xtrim(&key, &strategy, threshold, exact),
        ParsedCmd::Xdel { key, ids } => handle_xdel(&key, &ids),
        ParsedCmd::Xread { count, keys, ids } => handle_xread(count, &keys, &ids),
    }
}

fn handle_ping() -> resp::RespType {
    resp::RespType::SimpleString("PONG".to_string())
}

fn handle_echo(message: &str) -> resp::RespType {
    resp::RespType::BulkString(Some(Bytes::copy_from_slice(message.as_bytes())))
}

fn handle_set(key: &str, value: &str, expiry: Option<Duration>) -> resp::RespType {
    with_db(|db| {
        db.insert(
            key.to_string(),
            Entry::new(
                Value::String(Bytes::from(value.to_string())),
                expiry.map(|d| Instant::now() + d),
            ),
        );
    });
    resp::RespType::SimpleString("OK".to_string())
}

fn handle_get(key: &str) -> resp::RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => {
            if entry.expiry.is_some_and(|exp| Instant::now() >= exp) {
                db.remove(key);
                resp::RespType::BulkString(None)
            } else {
                match entry.value.clone() {
                    Value::String(v) => resp::RespType::BulkString(Some(v)),
                    _ => wrong_type(),
                }
            }
        }
        None => resp::RespType::BulkString(None),
    })
}

fn handle_rpush(key: &str, values: &[String]) -> resp::RespType {
    let values: VecDeque<Bytes> = values.iter().map(|v| Bytes::from(v.clone())).collect();
    let result = with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                list.extend(values);
                resp::RespType::Integer(list.len() as i64)
            } else {
                wrong_type()
            }
        }
        None => {
            let len = values.len();
            db.insert(key.to_string(), Entry::new(Value::List(values), None));
            resp::RespType::Integer(len as i64)
        }
    });
    blocking::notify_waiters(key);
    result
}

fn handle_lpush(key: &str, values: &[String]) -> resp::RespType {
    let values: VecDeque<Bytes> = values.iter().map(|v| Bytes::from(v.clone())).collect();
    let result = with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                for v in values {
                    list.push_front(v);
                }
                resp::RespType::Integer(list.len() as i64)
            } else {
                wrong_type()
            }
        }
        None => {
            let len = values.len();
            let mut list = VecDeque::new();
            for v in values {
                list.push_front(v);
            }
            db.insert(key.to_string(), Entry::new(Value::List(list), None));
            resp::RespType::Integer(len as i64)
        }
    });
    blocking::notify_waiters(key);
    result
}

fn handle_lrange(key: &str, start: i64, stop: i64) -> resp::RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match entry.value.clone() {
            Value::List(list) => {
                let len = list.len() as i64;
                if len == 0 {
                    return resp::RespType::Array(Some(vec![]));
                }

                let mut l = if start < 0 { len + start } else { start };
                let mut r = if stop < 0 { len + stop } else { stop };

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
            _ => wrong_type(),
        },
        None => resp::RespType::Array(Some(vec![])),
    })
}

fn handle_llen(key: &str) -> resp::RespType {
    with_db(|db| match db.get(key) {
        Some(v) => match &v.value {
            Value::List(u) => resp::RespType::Integer(u.len() as i64),
            _ => wrong_type(),
        },
        None => resp::RespType::Integer(0),
    })
}

fn handle_lpop(key: &str, count: Option<usize>) -> resp::RespType {
    if count == Some(0) {
        return resp::RespType::Array(Some(vec![]));
    }
    let n = count.unwrap_or(1);
    with_db(|db| match db.get_mut(key) {
        Some(entry) => {
            if let Value::List(ref mut list) = entry.value {
                let mut popped: Vec<resp::RespType> = Vec::new();
                for _ in 0..n {
                    match list.pop_front() {
                        Some(val) => popped.push(resp::RespType::BulkString(Some(val))),
                        None => break,
                    }
                }
                if list.is_empty() {
                    db.remove(key);
                }
                match count {
                    // No count arg -> single BulkString (like Redis LPOP)
                    None => popped
                        .into_iter()
                        .next()
                        .unwrap_or(resp::RespType::BulkString(None)),
                    // Count specified -> always Array
                    Some(_) if popped.is_empty() => resp::RespType::Array(None),
                    Some(_) => resp::RespType::Array(Some(popped)),
                }
            } else {
                wrong_type()
            }
        }
        None => match count {
            None => resp::RespType::BulkString(None),
            Some(_) => resp::RespType::Array(None),
        },
    })
}

/// Try to pop from the first non-empty list among keys.
/// Returns `Some(RespType)` if we should respond (success or WRONGTYPE error).
/// Returns `None` if no data is available (caller should block).
pub fn try_blpop(keys: &[String]) -> Option<resp::RespType> {
    with_db(|db| {
        for key in keys {
            match db.get_mut(key) {
                None => continue,
                Some(entry) => match &mut entry.value {
                    Value::List(list) => {
                        if let Some(val) = list.pop_front() {
                            if list.is_empty() {
                                db.remove(key);
                            }
                            return Some(resp::RespType::Array(Some(vec![
                                resp::RespType::BulkString(Some(Bytes::copy_from_slice(
                                    key.as_bytes(),
                                ))),
                                resp::RespType::BulkString(Some(val)),
                            ])));
                        }
                    }
                    _ => return Some(wrong_type()),
                },
            }
        }
        None
    })
}

async fn handle_blpop(keys: &[String], timeout: u64) -> resp::RespType {
    // First try — non-blocking
    if let Some(response) = try_blpop(keys) {
        return response;
    }

    // Blocking loop
    let notify = Arc::new(Notify::new());

    loop {
        let guard = with_db(|_| blocking::register(keys, &notify));

        if timeout == 0 {
            notify.notified().await;
        } else {
            let notified = notify.notified();
            tokio::pin!(notified);
            let timed_out = tokio::time::timeout(Duration::from_secs(timeout), notified)
                .await
                .is_err();
            if timed_out {
                drop(guard);
                return resp::RespType::Array(None);
            }
        }

        drop(guard);

        match try_blpop(keys) {
            Some(response) => return response,
            None => continue,
        }
    }
}

fn handle_command(subcommand: Option<String>, name: Option<String>) -> resp::RespType {
    match subcommand.as_deref() {
        Some("INFO") => {
            if let Some(n) = name {
                // COMMAND INFO <name>
                let info = registry::with_registry(|reg| {
                    reg.get(&n).map(|ci| {
                        let mut arr = Vec::new();
                        arr.push(resp::RespType::BulkString(Some(Bytes::copy_from_slice(
                            ci.name.as_bytes(),
                        ))));
                        arr.push(resp::RespType::Integer(ci.arity as i64));
                        arr.push(resp::RespType::Array(Some(vec![]))); // flags
                        arr.push(resp::RespType::Integer(0)); // first key
                        arr.push(resp::RespType::Integer(if ci.arity.abs() > 1 {
                            ci.arity.unsigned_abs() as i64 - 1
                        } else {
                            0
                        })); // last key
                        arr.push(resp::RespType::Integer(1)); // step
                        resp::RespType::Array(Some(arr))
                    })
                });
                match info {
                    Some(item) => resp::RespType::Array(Some(vec![item])),
                    None => resp::RespType::Array(None),
                }
            } else {
                // COMMAND INFO (without name) — return all
                let infos = registry::with_registry(|reg| {
                    reg.list_all()
                        .iter()
                        .map(|ci| {
                            let mut arr = Vec::new();
                            arr.push(resp::RespType::BulkString(Some(Bytes::copy_from_slice(
                                ci.name.as_bytes(),
                            ))));
                            arr.push(resp::RespType::Integer(ci.arity as i64));
                            arr.push(resp::RespType::Array(Some(vec![])));
                            arr.push(resp::RespType::Integer(0));
                            arr.push(resp::RespType::Integer(if ci.arity.abs() > 1 {
                                ci.arity.unsigned_abs() as i64 - 1
                            } else {
                                0
                            }));
                            arr.push(resp::RespType::Integer(1));
                            resp::RespType::Array(Some(arr))
                        })
                        .collect::<Vec<_>>()
                });
                resp::RespType::Array(Some(infos))
            }
        }
        _ => {
            // COMMAND (plain) — return list of command names only
            let names = registry::with_registry(|reg| {
                reg.list_all()
                    .iter()
                    .map(|ci| {
                        resp::RespType::BulkString(Some(Bytes::copy_from_slice(ci.name.as_bytes())))
                    })
                    .collect::<Vec<_>>()
            });
            resp::RespType::Array(Some(names))
        }
    }
}

fn handle_flushdb() -> resp::RespType {
    crate::db::flushdb();
    resp::RespType::SimpleString("OK".to_string())
}

// ── Stream ID helpers ─────────────────────────────────────────────────

fn parse_stream_id(id: &str) -> Option<(i64, u64)> {
    if id == "*" || id == "-" || id == "+" {
        return None;
    }
    let parts: Vec<&str> = id.splitn(2, '-').collect();
    if parts.len() != 2 {
        return None;
    }
    let ts = parts[0].parse::<i64>().ok()?;
    let seq = parts[1].parse::<u64>().ok()?;
    Some((ts, seq))
}

fn auto_stream_id(last_ts: i64, last_seq: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let (ts, seq) = if now > last_ts {
        (now, 0)
    } else if now == last_ts {
        (last_ts, last_seq + 1)
    } else {
        (last_ts + 1, 0)
    };
    format!("{}-{}", ts, seq)
}

fn auto_seq_for_timestamp(ts: i64, last_ts: i64, last_seq: u64) -> String {
    if ts > last_ts {
        format!("{}-{}", ts, 0)
    } else if ts == last_ts {
        format!("{}-{}", ts, last_seq + 1)
    } else {
        // Timestamp is behind — use last_timestamp + 1
        format!("{}-0", last_ts + 1)
    }
}

fn make_stream_entry(id: String, fields: Vec<(Bytes, Bytes)>) -> resp::RespType {
    let mut arr = vec![resp::RespType::BulkString(Some(Bytes::from(id)))];
    let mut fv = Vec::with_capacity(fields.len() * 2);
    for (k, v) in &fields {
        fv.push(resp::RespType::BulkString(Some(k.clone())));
        fv.push(resp::RespType::BulkString(Some(v.clone())));
    }
    arr.push(resp::RespType::Array(Some(fv)));
    resp::RespType::Array(Some(arr))
}

// ── Stream command handlers ───────────────────────────────────────────

fn handle_xadd(key: &str, id_spec: &str, field_args: &[String]) -> resp::RespType {
    let fields: Vec<(Bytes, Bytes)> = field_args
        .chunks(2)
        .map(|c| (Bytes::from(c[0].clone()), Bytes::from(c[1].clone())))
        .collect();

    with_db(|db| {
        let entry = db.entry(key.to_string()).or_insert_with(|| {
            Entry::new(Value::Stream(StreamData::new()), None)
        });

        match &mut entry.value {
            Value::Stream(stream) => {
                let is_empty = stream.entries.is_empty();
                let final_id = if id_spec == "*" {
                    auto_stream_id(stream.last_timestamp_ms, stream.last_seq)
                } else if let Some(ts) = id_spec.strip_suffix("-*") {
                    let t = ts.parse::<i64>().unwrap_or(0);
                    auto_seq_for_timestamp(t, stream.last_timestamp_ms, stream.last_seq)
                } else if let Some((ts, seq)) = parse_stream_id(id_spec) {
                    let last = (stream.last_timestamp_ms, stream.last_seq);
                    if !is_empty && (ts, seq) <= last {
                        return resp::RespType::Error(
                            "ERR The ID specified in XADD is equal or smaller than the target stream top item".to_string(),
                        );
                    }
                    format!("{}-{}", ts, seq)
                } else {
                    return resp::RespType::Error("ERR invalid stream ID".to_string());
                };

                // Update last used ID
                if let Some((ts, seq)) = parse_stream_id(&final_id) {
                    stream.last_timestamp_ms = ts;
                    stream.last_seq = seq;
                }

                stream.entries.push_back(StreamEntry {
                    id: final_id.clone(),
                    fields,
                });

                resp::RespType::BulkString(Some(Bytes::from(final_id)))
            }
            _ => wrong_type(),
        }
    })
}

fn handle_xrange(key: &str, start: &str, end: &str, count: Option<u64>) -> resp::RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Stream(stream) => {
                let start_id = if start == "-" {
                    (i64::MIN, 0u64)
                } else {
                    parse_stream_id(start).unwrap_or((i64::MIN, 0))
                };
                let end_id = if end == "+" {
                    (i64::MAX, u64::MAX)
                } else {
                    parse_stream_id(end).unwrap_or((i64::MAX, u64::MAX))
                };

                let matched: Vec<resp::RespType> = stream
                    .entries
                    .iter()
                    .filter(|e| {
                        parse_stream_id(&e.id)
                            .map(|(ts, seq)| (ts, seq) >= start_id && (ts, seq) <= end_id)
                            .unwrap_or(false)
                    })
                    .take(count.unwrap_or(u64::MAX) as usize)
                    .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                    .collect();

                resp::RespType::Array(Some(matched))
            }
            _ => wrong_type(),
        },
        None => resp::RespType::Array(Some(vec![])),
    })
}

fn handle_xrevrange(key: &str, end: &str, start: &str, count: Option<u64>) -> resp::RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Stream(stream) => {
                let end_id = if end == "+" {
                    (i64::MAX, u64::MAX)
                } else {
                    parse_stream_id(end).unwrap_or((i64::MAX, u64::MAX))
                };
                let start_id = if start == "-" {
                    (i64::MIN, 0u64)
                } else {
                    parse_stream_id(start).unwrap_or((i64::MIN, 0))
                };

                let matched: Vec<resp::RespType> = stream
                    .entries
                    .iter()
                    .rev()
                    .filter(|e| {
                        parse_stream_id(&e.id)
                            .map(|(ts, seq)| (ts, seq) >= start_id && (ts, seq) <= end_id)
                            .unwrap_or(false)
                    })
                    .take(count.unwrap_or(u64::MAX) as usize)
                    .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                    .collect();

                resp::RespType::Array(Some(matched))
            }
            _ => wrong_type(),
        },
        None => resp::RespType::Array(Some(vec![])),
    })
}

fn handle_xlen(key: &str) -> resp::RespType {
    with_db(|db| match db.get(key) {
        Some(entry) => match &entry.value {
            Value::Stream(stream) => resp::RespType::Integer(stream.entries.len() as i64),
            _ => wrong_type(),
        },
        None => resp::RespType::Integer(0),
    })
}

fn handle_xtrim(key: &str, _strategy: &str, threshold: u64, _exact: bool) -> resp::RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Stream(stream) => {
                let before = stream.entries.len();
                if before > threshold as usize {
                    let to_remove = before - threshold as usize;
                    stream.entries.drain(..to_remove);
                    // Reset last_ts/seq if everything was removed
                    if stream.entries.is_empty() {
                        stream.last_timestamp_ms = 0;
                        stream.last_seq = 0;
                    }
                    resp::RespType::Integer(to_remove as i64)
                } else {
                    resp::RespType::Integer(0)
                }
            }
            _ => wrong_type(),
        },
        None => resp::RespType::Integer(0),
    })
}

fn handle_xdel(key: &str, ids: &[String]) -> resp::RespType {
    with_db(|db| match db.get_mut(key) {
        Some(entry) => match &mut entry.value {
            Value::Stream(stream) => {
                let before = stream.entries.len();
                stream.entries.retain(|e| !ids.contains(&e.id));
                let removed = before - stream.entries.len();
                if stream.entries.is_empty() {
                    db.remove(key);
                }
                resp::RespType::Integer(removed as i64)
            }
            _ => wrong_type(),
        },
        None => resp::RespType::Integer(0),
    })
}

fn handle_xread(count: Option<u64>, keys: &[String], ids: &[String]) -> resp::RespType {
    with_db(|db| {
        let mut streams_resp: Vec<resp::RespType> = Vec::new();

        for (key, since_id_str) in keys.iter().zip(ids.iter()) {
            let since = parse_stream_id(since_id_str).unwrap_or((0, 0));

            if let Some(entry) = db.get(key) {
                if let Value::Stream(ref stream) = entry.value {
                    let entries: Vec<resp::RespType> = stream
                        .entries
                        .iter()
                        .filter(|e| {
                            parse_stream_id(&e.id)
                                .map(|(ts, seq)| (ts, seq) > since)
                                .unwrap_or(false)
                        })
                        .take(count.unwrap_or(u64::MAX) as usize)
                        .map(|e| make_stream_entry(e.id.clone(), e.fields.clone()))
                        .collect();

                    if !entries.is_empty() {
                        streams_resp.push(resp::RespType::Array(Some(vec![
                            resp::RespType::BulkString(Some(Bytes::from(key.clone()))),
                            resp::RespType::Array(Some(entries)),
                        ])));
                    }
                }
            }
        }

        if streams_resp.is_empty() {
            resp::RespType::Array(Some(vec![]))
        } else {
            resp::RespType::Array(Some(streams_resp))
        }
    })
}
