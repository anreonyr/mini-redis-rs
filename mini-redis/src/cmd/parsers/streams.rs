use super::super::types::{CmdError, ParsedCmd, XGroupSub, wrong_arg_count};

pub fn parse_stream_cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
        "XADD" => {
            if args.len() < 3 || args.len() % 2 != 0 {
                return Err(wrong_arg_count("xadd"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let id = iter.next().unwrap();
            let fields: Vec<String> = iter.collect();
            Ok(ParsedCmd::Xadd { key, id, fields })
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
            Ok(ParsedCmd::Xrange { key, start, end, count })
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
            Ok(ParsedCmd::Xrevrange { key, end, start, count })
        }
        "XLEN" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("xlen"))?;
            Ok(ParsedCmd::Xlen { key })
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
            Ok(ParsedCmd::Xtrim { key, strategy, threshold, exact })
        }
        "XDEL" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("xdel"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let ids: Vec<String> = iter.collect();
            Ok(ParsedCmd::Xdel { key, ids })
        }
        "XREAD" => {
            if args.is_empty() {
                return Err(wrong_arg_count("xread"));
            }
            let (count, remaining) = if args[0] == "COUNT" {
                if args.len() < 3 {
                    return Err(wrong_arg_count("xread"));
                }
                let n = args[1].parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
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
            Ok(ParsedCmd::Xread { count, keys, ids })
        }
        "XGROUP" => {
            if args.len() < 3 {
                return Err(wrong_arg_count("xgroup"));
            }
            let mut iter = args.into_iter();
            let sub = iter.next().unwrap().to_uppercase();
            let key = iter.next().unwrap();
            let sub = match sub.as_str() {
                "CREATE" => {
                    let group = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    let id = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    XGroupSub::Create { group, id }
                }
                "CREATECONSUMER" => {
                    let group = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    let consumer = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    XGroupSub::CreateConsumer { group, consumer }
                }
                "DELCONSUMER" => {
                    let group = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    let consumer = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    XGroupSub::DelConsumer { group, consumer }
                }
                "DESTROY" => {
                    let group = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    XGroupSub::Destroy { group }
                }
                "SETID" => {
                    let group = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    let id = iter.next().ok_or_else(|| wrong_arg_count("xgroup"))?;
                    XGroupSub::SetId { group, id }
                }
                _ => return Err(CmdError::SyntaxError),
            };
            Ok(ParsedCmd::XGroup { sub, key })
        }
        "XREADGROUP" => {
            if args.len() < 6 {
                return Err(wrong_arg_count("xreadgroup"));
            }
            let mut idx = 0;
            if args.get(idx).map(|s| s.as_str()) != Some("GROUP") {
                return Err(CmdError::SyntaxError);
            }
            idx += 1;
            let group = args.get(idx).ok_or_else(|| wrong_arg_count("xreadgroup"))?.clone();
            idx += 1;
            let consumer = args.get(idx).ok_or_else(|| wrong_arg_count("xreadgroup"))?.clone();
            idx += 1;

            let count = if args.get(idx).map(|s| s.as_str()) == Some("COUNT") {
                idx += 1;
                let n = args.get(idx)
                    .ok_or_else(|| wrong_arg_count("xreadgroup"))?
                    .parse::<u64>()
                    .map_err(|_| CmdError::InvalidInteger)?;
                idx += 1;
                Some(n)
            } else {
                None
            };

            if args.get(idx).map(|s| s.as_str()) != Some("STREAMS") {
                return Err(CmdError::SyntaxError);
            }
            idx += 1;

            let remaining = args[idx..].to_vec();
            if remaining.len() < 2 || remaining.len() % 2 != 0 {
                return Err(wrong_arg_count("xreadgroup"));
            }
            let mid = remaining.len() / 2;
            let keys = remaining[..mid].to_vec();
            let ids = remaining[mid..].to_vec();

            Ok(ParsedCmd::XReadGroup { group, consumer, count, keys, ids })
        }
        "XACK" => {
            if args.len() < 3 {
                return Err(wrong_arg_count("xack"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let group = iter.next().unwrap();
            let ids: Vec<String> = iter.collect();
            Ok(ParsedCmd::XAck { key, group, ids })
        }
        "XPENDING" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("xpending"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let group = iter.next().unwrap();
            let (start, end, count, consumer) = match iter.len() {
                0 => ("-".to_string(), "+".to_string(), 10u64, None),
                3 => {
                    let s = iter.next().unwrap();
                    let e = iter.next().unwrap();
                    let c = iter.next().unwrap().parse::<u64>()
                        .map_err(|_| CmdError::InvalidInteger)?;
                    (s, e, c, None)
                }
                4 => {
                    let s = iter.next().unwrap();
                    let e = iter.next().unwrap();
                    let c = iter.next().unwrap().parse::<u64>()
                        .map_err(|_| CmdError::InvalidInteger)?;
                    let con = iter.next().unwrap();
                    (s, e, c, Some(con))
                }
                _ => return Err(wrong_arg_count("xpending")),
            };
            Ok(ParsedCmd::XPending { key, group, start, end, count, consumer })
        }
        "XCLAIM" => {
            if args.len() < 5 {
                return Err(wrong_arg_count("xclaim"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let group = iter.next().unwrap();
            let consumer = iter.next().unwrap();
            let min_idle = iter.next().unwrap().parse::<u64>()
                .map_err(|_| CmdError::InvalidInteger)?;
            let ids: Vec<String> = iter.collect();
            Ok(ParsedCmd::XClaim { key, group, consumer, min_idle, ids })
        }
        "XINFO" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("xinfo"));
            }
            let mut iter = args.into_iter();
            let sub = iter.next().unwrap().to_uppercase();
            let key = iter.next().unwrap();
            let group = iter.next();
            Ok(ParsedCmd::XInfo { sub, key, group })
        }
        _ => Err(CmdError::UnknownCommand),
    }
}
