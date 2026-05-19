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
}
