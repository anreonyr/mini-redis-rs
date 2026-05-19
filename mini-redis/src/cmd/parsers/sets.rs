use super::super::types::{CmdError, ParsedCmd, wrong_arg_count};

pub fn parse_set_cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
        "SADD" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("sadd"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let members: Vec<String> = iter.collect();
            Ok(ParsedCmd::Sadd { key, members })
        }
        "SMEMBERS" => {
            if args.len() != 1 {
                return Err(wrong_arg_count("smembers"));
            }
            let key = args.into_iter().next().unwrap();
            Ok(ParsedCmd::Smembers { key })
        }
        "SISMEMBER" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("sismember"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let member = iter.next().unwrap();
            Ok(ParsedCmd::Sismember { key, member })
        }
        "SREM" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("srem"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let members: Vec<String> = iter.collect();
            Ok(ParsedCmd::Srem { key, members })
        }
        "SCARD" => {
            if args.len() != 1 {
                return Err(wrong_arg_count("scard"));
            }
            let key = args.into_iter().next().unwrap();
            Ok(ParsedCmd::Scard { key })
        }
        "SPOP" => {
            if args.is_empty() || args.len() > 2 {
                return Err(wrong_arg_count("spop"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let count = iter.next().map(|s| s.parse::<usize>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            Ok(ParsedCmd::Spop { key, count })
        }
        "SRANDMEMBER" => {
            if args.is_empty() || args.len() > 2 {
                return Err(wrong_arg_count("srandmember"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let count = iter.next().map(|s| s.parse::<i64>().map_err(|_| CmdError::InvalidInteger)).transpose()?;
            Ok(ParsedCmd::Srandmember { key, count })
        }
        "SUNION" => {
            if args.is_empty() {
                return Err(wrong_arg_count("sunion"));
            }
            Ok(ParsedCmd::Sunion { keys: args })
        }
        "SINTER" => {
            if args.is_empty() {
                return Err(wrong_arg_count("sinter"));
            }
            Ok(ParsedCmd::Sinter { keys: args })
        }
        "SDIFF" => {
            if args.is_empty() {
                return Err(wrong_arg_count("sdiff"));
            }
            Ok(ParsedCmd::Sdiff { keys: args })
        }
        "SMOVE" => {
            if args.len() != 3 {
                return Err(wrong_arg_count("smove"));
            }
            let mut iter = args.into_iter();
            let source = iter.next().unwrap();
            let destination = iter.next().unwrap();
            let member = iter.next().unwrap();
            Ok(ParsedCmd::Smove { source, destination, member })
        }
        _ => Err(CmdError::UnknownCommand),
    }
}
