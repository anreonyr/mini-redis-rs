use super::super::types::{CmdError, ParsedCmd, wrong_arg_count};

pub fn cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
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
            Ok(ParsedCmd::Hset { key, fields })
        }
        "HGET" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("hget"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let field = iter.next().unwrap();
            Ok(ParsedCmd::Hget { key, field })
        }
        "HDEL" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("hdel"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let fields: Vec<String> = iter.collect();
            Ok(ParsedCmd::Hdel { key, fields })
        }
        "HGETALL" => {
            if args.len() != 1 {
                return Err(wrong_arg_count("hgetall"));
            }
            let key = args.into_iter().next().unwrap();
            Ok(ParsedCmd::Hgetall { key })
        }
        "HEXISTS" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("hexists"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let field = iter.next().unwrap();
            Ok(ParsedCmd::Hexists { key, field })
        }
        "HLEN" => {
            if args.len() != 1 {
                return Err(wrong_arg_count("hlen"));
            }
            let key = args.into_iter().next().unwrap();
            Ok(ParsedCmd::Hlen { key })
        }
        "HKEYS" => {
            if args.len() != 1 {
                return Err(wrong_arg_count("hkeys"));
            }
            let key = args.into_iter().next().unwrap();
            Ok(ParsedCmd::Hkeys { key })
        }
        "HVALS" => {
            if args.len() != 1 {
                return Err(wrong_arg_count("hvals"));
            }
            let key = args.into_iter().next().unwrap();
            Ok(ParsedCmd::Hvals { key })
        }
        "HINCRBY" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("hincrby"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let field = iter.next().unwrap();
            let incr = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Hincrby { key, field, incr })
        }
        "HINCRBYFLOAT" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("hincrbyfloat"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let field = iter.next().unwrap();
            let incr = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Hincrbyfloat { key, field, incr })
        }
        "HSETNX" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("hsetnx"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let field = iter.next().unwrap();
            let value = iter.next().unwrap();
            Ok(ParsedCmd::Hsetnx { key, field, value })
        }
        "HRANDFIELD" => {
            if args.is_empty() || args.len() > 3 {
                return Err(wrong_arg_count("hrandfield"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let count = iter.next().map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            let mut withvalues = false;
            if let Some(flag) = iter.next() {
                if flag.to_uppercase() == "WITHVALUES" {
                    withvalues = true;
                } else {
                    return Err(CmdError::SyntaxError);
                }
            }
            Ok(ParsedCmd::Hrandfield { key, count, withvalues })
        }
        "HSTRLEN" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("hstrlen"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let field = iter.next().unwrap();
            Ok(ParsedCmd::Hstrlen { key, field })
        }
        _ => Err(CmdError::UnknownCommand),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_hset_ok() {
        let r = cmd("HSET", vec!["k".into(), "f1".into(), "v1".into()]);
        assert!(matches!(r, Ok(ParsedCmd::Hset { .. })));
    }
    #[test]
    fn test_hset_odd_args() {
        let r = cmd("HSET", vec!["k".into(), "f1".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_hget_ok() {
        let r = cmd("HGET", vec!["k".into(), "f".into()]);
        assert_eq!(r, Ok(ParsedCmd::Hget { key: "k".into(), field: "f".into() }));
    }
    #[test]
    fn test_hgetall_ok() {
        let r = cmd("HGETALL", vec!["k".into()]);
        assert_eq!(r, Ok(ParsedCmd::Hgetall { key: "k".into() }));
    }
    #[test]
    fn test_hrandfield_no_count() {
        let r = cmd("HRANDFIELD", vec!["k".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Hrandfield { key: "k".into(), count: None, withvalues: false })
        );
    }
    #[test]
    fn test_hrandfield_with_count() {
        let r = cmd("HRANDFIELD", vec!["k".into(), "3".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Hrandfield { key: "k".into(), count: Some(3), withvalues: false })
        );
    }
    #[test]
    fn test_hrandfield_negative_count() {
        let r = cmd("HRANDFIELD", vec!["k".into(), "-5".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Hrandfield { key: "k".into(), count: Some(-5), withvalues: false })
        );
    }
    #[test]
    fn test_hrandfield_withvalues() {
        let r = cmd("HRANDFIELD", vec!["k".into(), "2".into(), "WITHVALUES".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Hrandfield { key: "k".into(), count: Some(2), withvalues: true })
        );
    }
    #[test]
    fn test_hrandfield_wrong_arg_count() {
        let r = cmd("HRANDFIELD", vec![]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_hrandfield_too_many_args() {
        let r = cmd("HRANDFIELD", vec!["k".into(), "1".into(), "WITHVALUES".into(), "extra".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_hstrlen_ok() {
        let r = cmd("HSTRLEN", vec!["k".into(), "field".into()]);
        assert_eq!(
            r,
            Ok(ParsedCmd::Hstrlen { key: "k".into(), field: "field".into() })
        );
    }
    #[test]
    fn test_hstrlen_wrong_arg_count() {
        let r = cmd("HSTRLEN", vec!["k".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
    #[test]
    fn test_hstrlen_too_many_args() {
        let r = cmd("HSTRLEN", vec!["k".into(), "f1".into(), "f2".into()]);
        assert!(matches!(r, Err(CmdError::WrongArgCount(_))));
    }
}
