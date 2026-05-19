use super::super::types::{BitFieldSub, CmdError, ParsedCmd, wrong_arg_count};
use std::time::Duration;

pub fn cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
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
            Ok(ParsedCmd::Set { key, value, expiry })
        }
        "GET" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("get"))?;
            Ok(ParsedCmd::Get { key })
        }
        "INCR" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("incr"))?;
            Ok(ParsedCmd::Incr { key })
        }
        "DECR" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("decr"))?;
            Ok(ParsedCmd::Decr { key })
        }
        "INCRBY" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("incrby"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let delta = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Incrby { key, delta })
        }
        "DECRBY" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("decrby"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let delta = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Decrby { key, delta })
        }
        "APPEND" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("append"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Append { key, value })
        }
        "STRLEN" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("strlen"))?;
            Ok(ParsedCmd::Strlen { key })
        }
        "MGET" => {
            if args.is_empty() {
                return Err(wrong_arg_count("mget"));
            }
            Ok(ParsedCmd::Mget { keys: args })
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
            Ok(ParsedCmd::Mset { pairs })
        }
        "GETSET" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("getset"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Getset { key, value })
        }
        "GETRANGE" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("getrange"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let end = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Getrange { key, start, end })
        }
        "SETRANGE" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("setrange"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let offset = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Setrange { key, offset, value })
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
            Ok(ParsedCmd::Msetnx { pairs })
        }
        "SETNX" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("setnx"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Setnx { key, value })
        }
        "GETEX" => {
            if args.is_empty() || args.len() > 3 {
                return Err(wrong_arg_count("getex"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let expiry = match iter.next() {
                Some(flag) => match flag.as_str() {
                    "EX" => {
                        let val = iter.next().ok_or_else(|| wrong_arg_count("getex"))?;
                        Some(Duration::from_secs(val.parse().map_err(|_| CmdError::InvalidInteger)?))
                    }
                    "PX" => {
                        let val = iter.next().ok_or_else(|| wrong_arg_count("getex"))?;
                        Some(Duration::from_millis(val.parse().map_err(|_| CmdError::InvalidInteger)?))
                    }
                    "EXAT" => {
                        let val = iter.next().ok_or_else(|| wrong_arg_count("getex"))?;
                        let ts: u64 = val.parse().map_err(|_| CmdError::InvalidInteger)?;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        Some(Duration::from_secs(ts.saturating_sub(now)))
                    }
                    "PERSIST" => Some(Duration::ZERO),
                    _ => return Err(CmdError::SyntaxError),
                },
                None => None,
            };
            if iter.next().is_some() {
                return Err(wrong_arg_count("getex"));
            }
            Ok(ParsedCmd::Getex { key, expiry })
        }
        "GETDEL" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("getdel"))?;
            Ok(ParsedCmd::Getdel { key })
        }
        // Bitmap
        "GETBIT" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("getbit"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let offset = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::GetBit { key, offset })
        }
        "SETBIT" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("setbit"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let offset = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let val = iter.next().unwrap().parse::<u8>().map_err(|_| CmdError::InvalidInteger)?;
            if val > 1 { return Err(CmdError::InvalidInteger); }
            Ok(ParsedCmd::SetBit { key, offset, value: val })
        }
        "BITCOUNT" => {
            if args.is_empty() || args.len() > 3 {
                return Err(wrong_arg_count("bitcount"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            let end = iter.next().map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            Ok(ParsedCmd::BitCount { key, start, end })
        }
        "BITOP" => {
            if args.len() < 3 {
                return Err(wrong_arg_count("bitop"));
            }
            let mut iter = args.into_iter();
            let op = iter.next().unwrap().to_uppercase();
            let dest = iter.next().unwrap();
            let keys: Vec<String> = iter.collect();
            Ok(ParsedCmd::BitOp { op, dest, keys })
        }
        "BITPOS" => {
            if args.is_empty() || args.len() > 4 {
                return Err(wrong_arg_count("bitpos"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let bit = iter.next().unwrap().parse::<u8>().map_err(|_| CmdError::InvalidInteger)?;
            if bit > 1 { return Err(CmdError::InvalidInteger); }
            let start = iter.next().map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            let end = iter.next().map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            Ok(ParsedCmd::BitPos { key, bit, start, end })
        }
        "BITFIELD" | "BITFIELD_RO" => {
            let readonly = cmd == "BITFIELD_RO";
            if args.len() < 1 {
                return Err(wrong_arg_count(cmd));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let mut sub_commands = Vec::new();
            while let Some(token) = iter.next() {
                match token.to_uppercase().as_str() {
                    "GET" => {
                        let encoding = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let offset_str = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let offset = parse_bitfield_offset(&offset_str)?;
                        sub_commands.push(BitFieldSub::Get { encoding, offset });
                    }
                    "SET" => {
                        let encoding = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let offset_str = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let offset = parse_bitfield_offset(&offset_str)?;
                        let value = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?
                            .parse::<i64>().map_err(|_| CmdError::InvalidInteger)?;
                        sub_commands.push(BitFieldSub::Set { encoding, offset, value });
                    }
                    "INCRBY" => {
                        let encoding = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let offset_str = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let offset = parse_bitfield_offset(&offset_str)?;
                        let increment = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?
                            .parse::<i64>().map_err(|_| CmdError::InvalidInteger)?;
                        sub_commands.push(BitFieldSub::Incrby { encoding, offset, increment });
                    }
                    "OVERFLOW" => {
                        let behavior = iter.next().ok_or_else(|| wrong_arg_count("bitfield"))?;
                        let upper = behavior.to_uppercase();
                        if !matches!(upper.as_str(), "WRAP" | "SAT" | "FAIL") {
                            return Err(CmdError::SyntaxError);
                        }
                        sub_commands.push(BitFieldSub::Overflow { behavior: upper });
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
            }
            if sub_commands.is_empty() {
                return Err(wrong_arg_count("bitfield"));
            }
            if readonly {
                Ok(ParsedCmd::BitFieldRo { key, sub_commands })
            } else {
                Ok(ParsedCmd::BitField { key, sub_commands })
            }
        }
        _ => Err(CmdError::UnknownCommand),
    }
}

/// Parse a BITFIELD offset: plain integer like "0" or encoded like "#3" (multiplied by encoding width).
fn parse_bitfield_offset(s: &str) -> Result<i64, CmdError> {
    if let Some(suffix) = s.strip_prefix('#') {
        let multiplier: i64 = suffix.parse().map_err(|_| CmdError::InvalidInteger)?;
        Ok(multiplier)
    } else {
        s.parse().map_err(|_| CmdError::InvalidInteger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_set_ok() {
        let r = cmd("SET", vec!["k".into(), "v".into()]);
        assert_eq!(r, Ok(ParsedCmd::Set { key: "k".into(), value: "v".into(), expiry: None }));
    }
    #[test]
    fn test_set_ex() {
        let r = cmd("SET", vec!["k".into(), "v".into(), "EX".into(), "10".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Set { expiry: Some(_), .. })));
    }
    #[test]
    fn test_set_invalid_expiry_val() {
        let r = cmd("SET", vec!["k".into(), "v".into(), "EX".into(), "x".into()]);
        assert!(matches!(r, Err(CmdError::InvalidInteger)));
    }
    #[test]
    fn test_set_invalid_flag() {
        let r = cmd("SET", vec!["k".into(), "v".into(), "BAD".into(), "x".into()]);
        assert!(matches!(r, Err(CmdError::SyntaxError)));
    }
    #[test]
    fn test_set_wrong_arg_count() {
        let r = cmd("SET", vec!["k".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_get_ok() {
        let r = cmd("GET", vec!["k".into()]);
        assert_eq!(r, Ok(ParsedCmd::Get { key: "k".into() }));
    }
    #[test]
    fn test_get_missing_arg() {
        let r = cmd("GET", vec![]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_incr_ok() {
        let r = cmd("INCR", vec!["k".into()]);
        assert_eq!(r, Ok(ParsedCmd::Incr { key: "k".into() }));
    }
    #[test]
    fn test_bitop_ok() {
        let r = cmd("BITOP", vec!["AND".into(), "dest".into(), "a".into(), "b".into()]);
        assert!(matches!(r, Ok(ParsedCmd::BitOp { .. })));
    }
    #[test]
    fn test_bitop_missing_args() {
        let r = cmd("BITOP", vec!["AND".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
}
