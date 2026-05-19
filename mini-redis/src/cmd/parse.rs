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
            "INCR" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("incr"))?;
                ParsedCmd::Incr { key }
            }
            "DECR" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("decr"))?;
                ParsedCmd::Decr { key }
            }
            "INCRBY" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("incrby"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let delta: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Incrby { key, delta }
            }
            "DECRBY" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("decrby"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let delta: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Decrby { key, delta }
            }
            "APPEND" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("append"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let value = iter.next().unwrap();
                ParsedCmd::Append { key, value }
            }
            "STRLEN" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("strlen"))?;
                ParsedCmd::Strlen { key }
            }
            "MGET" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("mget"));
                }
                ParsedCmd::Mget { keys: args }
            }
            "MSET" => {
                if args.len() < 2 || args.len() % 2 != 0 {
                    return Err(wrong_arg_count("mset"));
                }
                let mut iter = args.into_iter();
                let mut pairs = Vec::new();
                while let Some(k) = iter.next() {
                    let v = iter.next().unwrap();
                    pairs.push((k, v));
                }
                ParsedCmd::Mset { pairs }
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
            "RPOP" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(wrong_arg_count("rpop"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let count = iter
                    .next()
                    .map(|s| s.parse::<usize>().map_err(|_| CmdError::InvalidInteger))
                    .transpose()?;
                ParsedCmd::Rpop { key, count }
            }
            "LINDEX" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("lindex"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let index: i64 = iter
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Lindex { key, index }
            }
            "LREM" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("lrem"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let count: i64 = iter
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(|_| CmdError::InvalidInteger)?;
                let value = iter.next().unwrap();
                ParsedCmd::Lrem { key, count, value }
            }
            "LTRIM" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("ltrim"));
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
                ParsedCmd::Ltrim { key, start, stop }
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
            "INFO" => {
                let section = args.into_iter().next();
                ParsedCmd::Info { section }
            }
            "CONFIG" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("config"));
                }
                let mut iter = args.into_iter();
                let sub = iter.next().unwrap().to_uppercase();
                match sub.as_str() {
                    "GET" => {
                        let parameter = iter.next().unwrap();
                        ParsedCmd::ConfigGet { parameter }
                    }
                    "SET" => {
                        let parameter = iter.next().unwrap();
                        let value = iter.next().ok_or_else(|| wrong_arg_count("config"))?;
                        ParsedCmd::ConfigSet { parameter, value }
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
            }
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
            "SPOP" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(wrong_arg_count("spop"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let count = iter
                    .next()
                    .map(|s| s.parse::<usize>().map_err(|_| CmdError::InvalidInteger))
                    .transpose()?;
                ParsedCmd::Spop { key, count }
            }
            "SRANDMEMBER" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(wrong_arg_count("srandmember"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let count = iter
                    .next()
                    .map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger))
                    .transpose()?;
                ParsedCmd::Srandmember { key, count }
            }
            "SUNION" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("sunion"));
                }
                ParsedCmd::Sunion { keys: args }
            }
            "SINTER" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("sinter"));
                }
                ParsedCmd::Sinter { keys: args }
            }
            "SDIFF" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("sdiff"));
                }
                ParsedCmd::Sdiff { keys: args }
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
            "ZREM" => {
                if args.len() < 2 {
                    return Err(wrong_arg_count("zrem"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let members: Vec<String> = iter.collect();
                ParsedCmd::Zrem { key, members }
            }
            "ZCARD" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("zcard"))?;
                ParsedCmd::Zcard { key }
            }
            "ZCOUNT" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("zcount"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let min = iter.next().unwrap();
                let max = iter.next().unwrap();
                ParsedCmd::Zcount { key, min, max }
            }
            "ZRANGEBYSCORE" => {
                if args.len() < 3 {
                    return Err(wrong_arg_count("zrangebyscore"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let min = iter.next().unwrap();
                let max = iter.next().unwrap();
                let mut withscores = false;
                let mut limit = None;
                while let Some(flag) = iter.next() {
                    match flag.to_uppercase().as_str() {
                        "WITHSCORES" => withscores = true,
                        "LIMIT" => {
                            let offset: usize = iter
                                .next()
                                .ok_or_else(|| wrong_arg_count("zrangebyscore"))?
                                .parse()
                                .map_err(|_| CmdError::InvalidInteger)?;
                            let count: usize = iter
                                .next()
                                .ok_or_else(|| wrong_arg_count("zrangebyscore"))?
                                .parse()
                                .map_err(|_| CmdError::InvalidInteger)?;
                            limit = Some((offset, count));
                        }
                        _ => return Err(CmdError::SyntaxError),
                    }
                }
                ParsedCmd::Zrangebyscore { key, min, max, withscores, limit }
            }
            "ZINCRBY" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("zincrby"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let incr: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let member = iter.next().unwrap();
                ParsedCmd::Zincrby { key, incr, member }
            }
            "ZREVRANGE" => {
                if args.len() < 3 || args.len() > 4 {
                    return Err(wrong_arg_count("zrevrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let start: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let stop: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let withscores = iter.next().map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
                ParsedCmd::Zrevrange { key, start, stop, withscores }
            }
            "ZREVRANK" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("zrevrank"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let member = iter.next().unwrap();
                ParsedCmd::Zrevrank { key, member }
            }
            // Key Management
            "DEL" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("del"));
                }
                ParsedCmd::Del { keys: args }
            }
            "EXISTS" => {
                if args.is_empty() {
                    return Err(wrong_arg_count("exists"));
                }
                ParsedCmd::Exists { keys: args }
            }
            "TYPE" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("type"))?;
                ParsedCmd::Type { key }
            }
            "KEYS" => {
                let pattern = args.into_iter().next().ok_or_else(|| wrong_arg_count("keys"))?;
                ParsedCmd::Keys { pattern }
            }
            "DBSIZE" => ParsedCmd::Dbsize,
            // Expiry Management
            "EXPIRE" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("expire"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let seconds: u64 = iter
                    .next()
                    .unwrap()
                    .parse()
                    .map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Expire { key, seconds }
            }
            "TTL" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("ttl"))?;
                ParsedCmd::Ttl { key }
            }
            "PERSIST" => {
                let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("persist"))?;
                ParsedCmd::Persist { key }
            }
            // More String
            "GETSET" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("getset"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let value = iter.next().unwrap();
                ParsedCmd::Getset { key, value }
            }
            "GETRANGE" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("getrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let start: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let end: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Getrange { key, start, end }
            }
            "SETRANGE" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("setrange"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let offset: u64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let value = iter.next().unwrap();
                ParsedCmd::Setrange { key, offset, value }
            }
            "MSETNX" => {
                if args.len() < 2 || args.len() % 2 != 0 {
                    return Err(wrong_arg_count("msetnx"));
                }
                let mut iter = args.into_iter();
                let mut pairs = Vec::new();
                while let Some(k) = iter.next() {
                    let v = iter.next().unwrap();
                    pairs.push((k, v));
                }
                ParsedCmd::Msetnx { pairs }
            }
            // More List
            "RPOPLPUSH" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("rpoplpush"));
                }
                let mut iter = args.into_iter();
                let source = iter.next().unwrap();
                let destination = iter.next().unwrap();
                ParsedCmd::Rpoplpush { source, destination }
            }
            "LSET" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("lset"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let index: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let value = iter.next().unwrap();
                ParsedCmd::Lset { key, index, value }
            }
            // More Hash
            "HINCRBY" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("hincrby"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let field = iter.next().unwrap();
                let incr: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Hincrby { key, field, incr }
            }
            "HINCRBYFLOAT" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("hincrbyfloat"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let field = iter.next().unwrap();
                let incr: f64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Hincrbyfloat { key, field, incr }
            }
            "HSETNX" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("hsetnx"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let field = iter.next().unwrap();
                let value = iter.next().unwrap();
                ParsedCmd::Hsetnx { key, field, value }
            }
            // More Set
            "SMOVE" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("smove"));
                }
                let mut iter = args.into_iter();
                let source = iter.next().unwrap();
                let destination = iter.next().unwrap();
                let member = iter.next().unwrap();
                ParsedCmd::Smove { source, destination, member }
            }
            // More ZSet
            "ZREMRANGEBYRANK" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("zremrangebyrank"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let start: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                let stop: i64 = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
                ParsedCmd::Zremrangebyrank { key, start, stop }
            }
            "ZREMRANGEBYSCORE" => {
                if args.len() != 3 {
                    return Err(wrong_arg_count("zremrangebyscore"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let min = iter.next().unwrap();
                let max = iter.next().unwrap();
                ParsedCmd::Zremrangebyscore { key, min, max }
            }
            "ZREVRANGEBYSCORE" => {
                if args.len() < 3 {
                    return Err(wrong_arg_count("zrevrangebyscore"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let max = iter.next().unwrap();
                let min = iter.next().unwrap();
                let mut withscores = false;
                let mut limit = None;
                while let Some(flag) = iter.next() {
                    match flag.to_uppercase().as_str() {
                        "WITHSCORES" => withscores = true,
                        "LIMIT" => {
                            let offset: usize = iter.next().ok_or_else(|| wrong_arg_count("zrevrangebyscore"))?
                                .parse().map_err(|_| CmdError::InvalidInteger)?;
                            let count: usize = iter.next().ok_or_else(|| wrong_arg_count("zrevrangebyscore"))?
                                .parse().map_err(|_| CmdError::InvalidInteger)?;
                            limit = Some((offset, count));
                        }
                        _ => return Err(CmdError::SyntaxError),
                    }
                }
                ParsedCmd::Zrevrangebyscore { key, max, min, withscores, limit }
            }
            // More Key
            "RENAME" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("rename"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let newkey = iter.next().unwrap();
                ParsedCmd::Rename { key, newkey }
            }
            "RENAMENX" => {
                if args.len() != 2 {
                    return Err(wrong_arg_count("renamenx"));
                }
                let mut iter = args.into_iter();
                let key = iter.next().unwrap();
                let newkey = iter.next().unwrap();
                ParsedCmd::Renamenx { key, newkey }
            }
            "RANDOMKEY" => ParsedCmd::Randomkey,
            "AUTH" => {
                let password = args
                    .into_iter()
                    .next()
                    .ok_or_else(|| wrong_arg_count("auth"))?;
                ParsedCmd::Auth { password }
            }
            "SAVE" => ParsedCmd::Save,
            "BGSAVE" => ParsedCmd::Bgsave,
            "SHUTDOWN" => ParsedCmd::Shutdown,
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
