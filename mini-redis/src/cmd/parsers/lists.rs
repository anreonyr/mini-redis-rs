use super::super::types::{CmdError, ParsedCmd, wrong_arg_count};

pub fn cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
        "RPUSH" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("rpush"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let values: Vec<String> = iter.collect();
            Ok(ParsedCmd::Rpush { key, values })
        }
        "LPUSH" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("lpush"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let values: Vec<String> = iter.collect();
            Ok(ParsedCmd::Lpush { key, values })
        }
        "LRANGE" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("lrange"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let stop = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Lrange { key, start, stop })
        }
        "LLEN" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("llen"))?;
            Ok(ParsedCmd::Llen { key })
        }
        "LPOP" => {
            if args.is_empty() || args.len() > 2 {
                return Err(wrong_arg_count("lpop"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let count = iter.next().map(|s| s.parse::<usize>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            Ok(ParsedCmd::Lpop { key, count })
        }
        "RPOP" => {
            if args.is_empty() || args.len() > 2 {
                return Err(wrong_arg_count("rpop"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let count = iter.next().map(|s| s.parse::<usize>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            Ok(ParsedCmd::Rpop { key, count })
        }
        "LINDEX" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("lindex"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let index = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Lindex { key, index })
        }
        "LREM" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("lrem"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let count = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Lrem { key, count, value })
        }
        "LTRIM" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("ltrim"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let start = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let stop = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Ltrim { key, start, stop })
        }
        "BLPOP" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("blpop"));
            }
            let timeout = args[args.len() - 1].parse().map_err(|_| CmdError::InvalidInteger)?;
            let mut a = args;
            a.pop();
            Ok(ParsedCmd::Blpop { keys: a, timeout })
        }
        "BRPOP" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("brpop"));
            }
            let timeout = args[args.len() - 1].parse().map_err(|_| CmdError::InvalidInteger)?;
            let mut a = args;
            a.pop();
            Ok(ParsedCmd::Brpop { keys: a, timeout })
        }
        "RPOPLPUSH" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("rpoplpush"));
            }
            let mut iter = args.into_iter();
            let source = iter.next().unwrap();
            let destination = iter.next().unwrap();
            Ok(ParsedCmd::Rpoplpush { source, destination })
        }
        "LSET" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("lset"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let index = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Lset { key, index, value })
        }
        "BRPOPLPUSH" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("brpoplpush"));
            }
            let mut iter = args.into_iter();
            let source = iter.next().unwrap();
            let destination = iter.next().unwrap();
            let timeout = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Brpoplpush { source, destination, timeout })
        }
        "LMOVE" => {
            if args.len() != 4 {
                return Err(wrong_arg_count("lmove"));
            }
            let mut iter = args.into_iter();
            let source = iter.next().unwrap();
            let destination = iter.next().unwrap();
            let from_where = iter.next().unwrap().to_uppercase();
            let to_where = iter.next().unwrap().to_uppercase();
            if !matches!(from_where.as_str(), "LEFT" | "RIGHT") {
                return Err(CmdError::SyntaxError);
            }
            if !matches!(to_where.as_str(), "LEFT" | "RIGHT") {
                return Err(CmdError::SyntaxError);
            }
            Ok(ParsedCmd::Lmove { source, destination, from_where, to_where })
        }
        "BLMOVE" => {
            if args.len() != 5 {
                return Err(wrong_arg_count("blmove"));
            }
            let mut iter = args.into_iter();
            let source = iter.next().unwrap();
            let destination = iter.next().unwrap();
            let from_where = iter.next().unwrap().to_uppercase();
            let to_where = iter.next().unwrap().to_uppercase();
            if !matches!(from_where.as_str(), "LEFT" | "RIGHT") {
                return Err(CmdError::SyntaxError);
            }
            if !matches!(to_where.as_str(), "LEFT" | "RIGHT") {
                return Err(CmdError::SyntaxError);
            }
            let timeout = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Blmove { source, destination, from_where, to_where, timeout })
        }
        "LPOS" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("lpos"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let element = iter.next().unwrap();
            let mut rank = None;
            let mut count = None;
            let mut maxlen = None;
            let remaining: Vec<String> = iter.collect();
            let mut i = 0;
            while i < remaining.len() {
                match remaining[i].to_uppercase().as_str() {
                    "RANK" => {
                        i += 1;
                        if i >= remaining.len() {
                            return Err(CmdError::SyntaxError);
                        }
                        rank = Some(remaining[i].parse().map_err(|_| CmdError::InvalidInteger)?);
                    }
                    "COUNT" => {
                        i += 1;
                        if i >= remaining.len() {
                            return Err(CmdError::SyntaxError);
                        }
                        count = Some(remaining[i].parse().map_err(|_| CmdError::InvalidInteger)?);
                    }
                    "MAXLEN" => {
                        i += 1;
                        if i >= remaining.len() {
                            return Err(CmdError::SyntaxError);
                        }
                        maxlen = Some(remaining[i].parse().map_err(|_| CmdError::InvalidInteger)?);
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
                i += 1;
            }
            Ok(ParsedCmd::Lpos { key, element, rank, count, maxlen })
        }
        _ => Err(CmdError::UnknownCommand),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_rpush_ok() {
        let r = cmd("RPUSH", vec!["k".into(), "a".into(), "b".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Rpush { .. })));
    }
    #[test]
    fn test_lrange_ok() {
        let r = cmd("LRANGE", vec!["k".into(), "0".into(), "-1".into()]);
        assert_eq!(r, Ok(ParsedCmd::Lrange { key: "k".into(), start: 0, stop: -1 }));
    }
    #[test]
    fn test_lrange_invalid_start() {
        let r = cmd("LRANGE", vec!["k".into(), "x".into(), "-1".into()]);
        assert!(matches!(r, Err(CmdError::InvalidInteger)));
    }
    #[test]
    fn test_blpop_ok() {
        let r = cmd("BLPOP", vec!["k1".into(), "k2".into(), "5".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Blpop { keys, timeout }) if keys == vec!["k1", "k2"] && timeout == 5));
    }
    #[test]
    fn test_blpop_missing_key() {
        let r = cmd("BLPOP", vec!["k1".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_brpoplpush_ok() {
        let r = cmd("BRPOPLPUSH", vec!["src".into(), "dst".into(), "10".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Brpoplpush {
                source: "src".into(),
                destination: "dst".into(),
                timeout: 10
            })
        );
    }
    #[test]
    fn test_lmove_ok() {
        let r = cmd("LMOVE", vec!["src".into(), "dst".into(), "LEFT".into(), "RIGHT".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Lmove { .. })));
    }
    #[test]
    fn test_lmove_invalid_where() {
        let r = cmd("LMOVE", vec!["src".into(), "dst".into(), "UP".into(), "DOWN".into()]);
        assert!(matches!(r, Err(CmdError::SyntaxError)));
    }
    #[test]
    fn test_blmove_ok() {
        let r = cmd("BLMOVE", vec!["src".into(), "dst".into(), "LEFT".into(), "RIGHT".into(), "5".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Blmove { .. })));
    }
    #[test]
    fn test_lpos_ok() {
        let r = cmd("LPOS", vec!["k".into(), "v".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Lpos { .. })));
    }
    #[test]
    fn test_lpos_with_options() {
        let r = cmd("LPOS", vec!["k".into(), "v".into(), "RANK".into(), "2".into(), "COUNT".into(), "3".into(), "MAXLEN".into(), "10".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Lpos {
                key: "k".into(),
                element: "v".into(),
                rank: Some(2),
                count: Some(3),
                maxlen: Some(10)
            })
        );
    }
    #[test]
    fn test_lpos_invalid_rank() {
        let r = cmd("LPOS", vec!["k".into(), "v".into(), "RANK".into(), "x".into()]);
        assert!(matches!(r, Err(CmdError::InvalidInteger)));
    }
    #[test]
    fn test_lpos_trailing_flag_no_value() {
        let r = cmd("LPOS", vec!["k".into(), "v".into(), "COUNT".into()]);
        assert!(matches!(r, Err(CmdError::SyntaxError)));
    }
}
