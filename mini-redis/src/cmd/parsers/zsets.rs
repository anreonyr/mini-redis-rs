use super::super::types::{CmdError, ParsedCmd, wrong_arg_count};

pub fn cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
        "ZADD" => {
            if args.len() < 3 || args.len() % 2 == 0 {
                return Err(wrong_arg_count("zadd"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let mut members = Vec::new();
            while let Some(score_str) = iter.next() {
                let score = score_str.parse().map_err(|_| CmdError::InvalidInteger)?;
                let member = iter.next().unwrap();
                members.push((score, member));
            }
            Ok(ParsedCmd::Zadd { key, members })
        }
        "ZRANGE" => {
            if args.len() < 3 || args.len() > 4 {
                return Err(wrong_arg_count("zrange"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let stop = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let withscores = iter.next().map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
            Ok(ParsedCmd::Zrange { key, start, stop, withscores })
        }
        "ZRANK" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("zrank"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let member = iter.next().unwrap();
            Ok(ParsedCmd::Zrank { key, member })
        }
        "ZSCORE" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("zscore"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let member = iter.next().unwrap();
            Ok(ParsedCmd::Zscore { key, member })
        }
        "ZREM" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("zrem"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let members: Vec<String> = iter.collect();
            Ok(ParsedCmd::Zrem { key, members })
        }
        "ZCARD" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("zcard"))?;
            Ok(ParsedCmd::Zcard { key })
        }
        "ZCOUNT" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("zcount"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let min = iter.next().unwrap();
            let max = iter.next().unwrap();
            Ok(ParsedCmd::Zcount { key, min, max })
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
                        let offset = iter.next().ok_or_else(|| wrong_arg_count("zrangebyscore"))?
                            .parse().map_err(|_| CmdError::InvalidInteger)?;
                        let count = iter.next().ok_or_else(|| wrong_arg_count("zrangebyscore"))?
                            .parse().map_err(|_| CmdError::InvalidInteger)?;
                        limit = Some((offset, count));
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
            }
            Ok(ParsedCmd::Zrangebyscore { key, min, max, withscores, limit })
        }
        "ZINCRBY" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("zincrby"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let incr = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let member = iter.next().unwrap();
            Ok(ParsedCmd::Zincrby { key, incr, member })
        }
        "ZREVRANGE" => {
            if args.len() < 3 || args.len() > 4 {
                return Err(wrong_arg_count("zrevrange"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let stop = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let withscores = iter.next().map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false);
            Ok(ParsedCmd::Zrevrange { key, start, stop, withscores })
        }
        "ZREVRANK" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("zrevrank"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let member = iter.next().unwrap();
            Ok(ParsedCmd::Zrevrank { key, member })
        }
        "ZREMRANGEBYRANK" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("zremrangebyrank"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let stop = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Zremrangebyrank { key, start, stop })
        }
        "ZREMRANGEBYSCORE" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("zremrangebyscore"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let min = iter.next().unwrap();
            let max = iter.next().unwrap();
            Ok(ParsedCmd::Zremrangebyscore { key, min, max })
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
                        let offset = iter.next().ok_or_else(|| wrong_arg_count("zrevrangebyscore"))?
                            .parse().map_err(|_| CmdError::InvalidInteger)?;
                        let count = iter.next().ok_or_else(|| wrong_arg_count("zrevrangebyscore"))?
                            .parse().map_err(|_| CmdError::InvalidInteger)?;
                        limit = Some((offset, count));
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
            }
            Ok(ParsedCmd::Zrevrangebyscore { key, max, min, withscores, limit })
        }
        // ZSet Set Operations
        "ZINTERSTORE" | "ZUNIONSTORE" => {
            if args.len() < 3 {
                return Err(wrong_arg_count(cmd));
            }
            let dest = args[0].clone();
            let is_store = cmd == "ZINTERSTORE";
            let (numkeys, keys, weights, aggregate) = parse_zset_store_args(&args[1..])?;
            if is_store {
                Ok(ParsedCmd::ZInterStore { dest, numkeys, keys, weights, aggregate })
            } else {
                Ok(ParsedCmd::ZUnionStore { dest, numkeys, keys, weights, aggregate })
            }
        }
        "ZINTER" | "ZUNION" => {
            if args.len() < 2 {
                return Err(wrong_arg_count(cmd));
            }
            let mut iter = args.into_iter();
            let numkeys = iter.next().ok_or_else(|| wrong_arg_count(cmd))?
                .parse::<usize>().map_err(|_| CmdError::InvalidInteger)?;
            let mut keys = Vec::new();
            for _ in 0..numkeys {
                keys.push(iter.next().ok_or_else(|| wrong_arg_count(cmd))?);
            }
            let mut weights = Vec::new();
            let mut aggregate = "SUM".to_string();
            let mut withscores = false;
            while let Some(flag) = iter.next() {
                match flag.to_uppercase().as_str() {
                    "WEIGHTS" => {
                        for _ in 0..numkeys {
                            let w = iter.next().ok_or_else(|| wrong_arg_count(cmd))?
                                .parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
                            weights.push(w);
                        }
                    }
                    "AGGREGATE" => {
                        aggregate = iter.next().ok_or_else(|| wrong_arg_count(cmd))?.to_uppercase();
                        if !matches!(aggregate.as_str(), "SUM" | "MIN" | "MAX") {
                            return Err(CmdError::SyntaxError);
                        }
                    }
                    "WITHSCORES" => withscores = true,
                    _ => return Err(CmdError::SyntaxError),
                }
            }
            if cmd == "ZINTER" {
                Ok(ParsedCmd::ZInter { numkeys, keys, weights, aggregate, withscores })
            } else {
                Ok(ParsedCmd::ZUnion { numkeys, keys, weights, aggregate, withscores })
            }
        }
        "ZDIFF" | "ZDIFFSTORE" => {
            if args.len() < 2 {
                return Err(wrong_arg_count(cmd));
            }
            let is_store = cmd == "ZDIFFSTORE";
            let mut idx = 0;
            let dest = if is_store {
                let d = args.get(idx).ok_or_else(|| wrong_arg_count(cmd))?.clone();
                idx += 1;
                Some(d)
            } else { None };
            let numkeys = args.get(idx).ok_or_else(|| wrong_arg_count(cmd))?
                .parse::<usize>().map_err(|_| CmdError::InvalidInteger)?;
            idx += 1;
            let mut keys = Vec::new();
            for _ in 0..numkeys {
                keys.push(args.get(idx).ok_or_else(|| wrong_arg_count(cmd))?.clone());
                idx += 1;
            }
            let withscores = if !is_store {
                args.get(idx).map(|s| s.to_uppercase() == "WITHSCORES").unwrap_or(false)
            } else { false };
            if is_store {
                Ok(ParsedCmd::ZDiffStore { dest: dest.unwrap(), keys })
            } else {
                Ok(ParsedCmd::ZDiff { numkeys, keys, withscores })
            }
        }
        _ => Err(CmdError::UnknownCommand),
    }
}

/// Parse store-command arguments: numkeys key [key...] [WEIGHTS w...] [AGGREGATE SUM|MIN|MAX]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_zadd_ok() {
        let r = cmd("ZADD", vec!["k".into(), "1".into(), "a".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Zadd { .. })));
    }
    #[test]
    fn test_zadd_odd_args() {
        let r = cmd("ZADD", vec!["k".into(), "1".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_zrange_ok() {
        let r = cmd("ZRANGE", vec!["k".into(), "0".into(), "-1".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Zrange { .. })));
    }
    #[test]
    fn test_zinterstore_ok() {
        let r = cmd("ZINTERSTORE", vec!["dest".into(), "2".into(), "a".into(), "b".into()]);
        assert!(matches!(r, Ok(ParsedCmd::ZInterStore { dest, .. }) if dest == "dest"));
    }
    #[test]
    fn test_zunion_with_weights() {
        let r = cmd("ZUNION", vec!["2".into(), "a".into(), "b".into(), "WEIGHTS".into(), "1".into(), "2".into()]);
        assert!(matches!(r, Ok(ParsedCmd::ZUnion { .. })));
    }
    #[test]
    fn test_zdiff_ok() {
        let r = cmd("ZDIFF", vec!["2".into(), "a".into(), "b".into()]);
        assert!(matches!(r, Ok(ParsedCmd::ZDiff { .. })));
    }
}

fn parse_zset_store_args(args: &[String]) -> Result<(usize, Vec<String>, Vec<f64>, String), CmdError> {
    let mut iter = args.iter();
    let numkeys = iter.next().ok_or_else(|| wrong_arg_count("zstore"))?
        .parse::<usize>().map_err(|_| CmdError::InvalidInteger)?;
    let mut keys = Vec::new();
    for _ in 0..numkeys {
        keys.push(iter.next().ok_or_else(|| wrong_arg_count("zstore"))?.clone());
    }
    let mut weights = Vec::new();
    let mut aggregate = "SUM".to_string();
    while let Some(flag) = iter.next() {
        match flag.to_uppercase().as_str() {
            "WEIGHTS" => {
                for _ in 0..numkeys {
                    let w = iter.next().ok_or_else(|| wrong_arg_count("zstore"))?
                        .parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
                    weights.push(w);
                }
            }
            "AGGREGATE" => {
                aggregate = iter.next().ok_or_else(|| wrong_arg_count("zstore"))?.to_uppercase();
                if !matches!(aggregate.as_str(), "SUM" | "MIN" | "MAX") {
                    return Err(CmdError::SyntaxError);
                }
            }
            _ => return Err(CmdError::SyntaxError),
        }
    }
    Ok((numkeys, keys, weights, aggregate))
}
