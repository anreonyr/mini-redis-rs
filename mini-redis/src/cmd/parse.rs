use crate::resp;
use std::time::Duration;

use super::types::{CmdError, ParsedCmd, wrong_arg_count};

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
            // Hash
            "HSET" => {
                if args.len() < 3 || args.len() % 2 == 0 {
                    return Err(wrong_arg_count("hset"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let mut fields = Vec::new();
                while let Some(f) = iter.next() {
                    let v = iter.next().unwrap();
                    fields.push((f, v));
                }
                ParsedCmd::Hset { key, fields }
            }
            "HGET" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("hget"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let field = iter.next().unwrap();
                ParsedCmd::Hget { key, field }
            }
            "HDEL" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("hdel"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let fields: Vec<String> = iter.collect();
                ParsedCmd::Hdel { key, fields }
            }
            "HGETALL" => {
                if args.len() != 1 {
                    return Err(wrong_arg_count("hgetall"));
                }
                let key = args.into_iter().next().unwrap();
                ParsedCmd::Hgetall { key }
            }
            "HEXISTS" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("hexists"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let field = iter.next().unwrap();
                ParsedCmd::Hexists { key, field }
            }
            "HLEN" => {
                if args.len() != 1 {
                    return Err(wrong_arg_count("hlen"));
                }
                let key = args.into_iter().next().unwrap();
                ParsedCmd::Hlen { key }
            }
            "HKEYS" => {
                if args.len() != 1 {
                    return Err(wrong_arg_count("hkeys"));
                }
                let key = args.into_iter().next().unwrap();
                ParsedCmd::Hkeys { key }
            }
            "HVALS" => {
                if args.len() != 1 {
                    return Err(wrong_arg_count("hvals"));
                }
                let key = args.into_iter().next().unwrap();
                ParsedCmd::Hvals { key }
            }
            // Set
            "SADD" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("sadd"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let members: Vec<String> = iter.collect();
                ParsedCmd::Sadd { key, members }
            }
            "SMEMBERS" => {
                if args.len() != 1 {
                    return Err(wrong_arg_count("smembers"));
                }
                let key = args.into_iter().next().unwrap();
                ParsedCmd::Smembers { key }
            }
            "SISMEMBER" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("sismember"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let member = iter.next().unwrap();
                ParsedCmd::Sismember { key, member }
            }
            "SREM" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("srem"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let members: Vec<String> = iter.collect();
                ParsedCmd::Srem { key, members }
            }
            "SCARD" => {
                if args.len() != 1 {
                    return Err(wrong_arg_count("scard"));
                }
                let key = args.into_iter().next().unwrap();
                ParsedCmd::Scard { key }
            }
            // Sorted Set
            "ZADD" => {
                if args.len() < 3 || args.len() % 2 == 0 {
                    return Err(wrong_arg_count("zadd"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let mut members = Vec::new();
                while let Some(score_str) = iter.next() {
                    let score: i64 = score_str.parse().map_err(|_| CmdError::InvalidInteger)?;
                    let member = iter.next().unwrap();
                    members.push((score, member));
                }
                ParsedCmd::Zadd { key, members }
            }
            "ZRANGE" => {
                if args.len() < 3 || args.len() > 4 {
                    return Err(wrong_arg_count("zrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let start: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let stop: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let withscores = iter.next().map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
                ParsedCmd::Zrange { key, start, stop, withscores }
            }
            "ZRANK" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("zrank"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let member = iter.next().unwrap();
                ParsedCmd::Zrank { key, member }
            }
            "ZSCORE" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("zscore"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let member = iter.next().unwrap();
                ParsedCmd::Zscore { key, member }
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
