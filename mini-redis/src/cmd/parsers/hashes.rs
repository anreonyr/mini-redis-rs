use super::super::types::{CmdError, ParsedCmd, wrong_arg_count};

pub fn parse_hash_cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
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
        _ => Err(CmdError::UnknownCommand),
    }
}
