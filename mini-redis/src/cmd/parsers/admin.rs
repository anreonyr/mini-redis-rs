use super::super::types::{CmdError, ParsedCmd, wrong_arg_count};

pub fn cmd(cmd: &str, args: Vec<String>) -> Result<ParsedCmd, CmdError> {
    match cmd {
        "PING" => Ok(ParsedCmd::Ping),
        "ECHO" => {
            let message = args.into_iter().next().ok_or_else(|| wrong_arg_count("echo"))?;
            Ok(ParsedCmd::Echo { message })
        }
        "FLUSHDB" => Ok(ParsedCmd::Flushdb),
        "COMMAND" => {
            let mut iter = args.into_iter();
            let subcommand = iter.next().map(|s| s.to_uppercase());
            let name = iter.next();
            Ok(ParsedCmd::Command { subcommand, name })
        }
        "INFO" => {
            let section = args.into_iter().next();
            Ok(ParsedCmd::Info { section })
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
                    Ok(ParsedCmd::ConfigGet { parameter })
                }
                "SET" => {
                    let parameter = iter.next().unwrap();
                    let value = iter.next().ok_or_else(|| wrong_arg_count("config"))?;
                    Ok(ParsedCmd::ConfigSet { parameter, value })
                }
                _ => Err(CmdError::SyntaxError),
            }
        }
        // Key management
        "DEL" => {
            if args.is_empty() {
                return Err(wrong_arg_count("del"));
            }
            Ok(ParsedCmd::Del { keys: args })
        }
        "EXISTS" => {
            if args.is_empty() {
                return Err(wrong_arg_count("exists"));
            }
            Ok(ParsedCmd::Exists { keys: args })
        }
        "TYPE" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("type"))?;
            Ok(ParsedCmd::Type { key })
        }
        "KEYS" => {
            let pattern = args.into_iter().next().ok_or_else(|| wrong_arg_count("keys"))?;
            Ok(ParsedCmd::Keys { pattern })
        }
        "DBSIZE" => Ok(ParsedCmd::Dbsize),
        "RENAME" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("rename"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let newkey = iter.next().unwrap();
            Ok(ParsedCmd::Rename { key, newkey })
        }
        "RENAMENX" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("renamenx"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let newkey = iter.next().unwrap();
            Ok(ParsedCmd::Renamenx { key, newkey })
        }
        "RANDOMKEY" => Ok(ParsedCmd::Randomkey),
        // Expiry management
        "EXPIRE" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("expire"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let seconds = iter.next().unwrap().parse().map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Expire { key, seconds })
        }
        "TTL" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("ttl"))?;
            Ok(ParsedCmd::Ttl { key })
        }
        "PERSIST" => {
            let key = args.into_iter().next().ok_or_else(|| wrong_arg_count("persist"))?;
            Ok(ParsedCmd::Persist { key })
        }
        // Auth
        "AUTH" => {
            let password = args.into_iter().next().ok_or_else(|| wrong_arg_count("auth"))?;
            Ok(ParsedCmd::Auth { password })
        }
        // Server
        "SAVE" => Ok(ParsedCmd::Save),
        "BGSAVE" => Ok(ParsedCmd::Bgsave),
        "SHUTDOWN" => Ok(ParsedCmd::Shutdown),
        // Connection
        "SELECT" => {
            let index = args.into_iter().next()
                .ok_or_else(|| wrong_arg_count("select"))?
                .parse::<usize>()
                .map_err(|_| CmdError::InvalidInteger)?;
            Ok(ParsedCmd::Select { index })
        }
        "QUIT" => Ok(ParsedCmd::Quit),
        "CLIENT" => {
            let sub = args.first().map(|s| s.to_uppercase());
            match sub.as_deref() {
                Some("SETNAME") => {
                    let name = args.get(1).ok_or_else(|| wrong_arg_count("client setname"))?;
                    Ok(ParsedCmd::ClientSetName { name: name.clone() })
                }
                Some("GETNAME") => Ok(ParsedCmd::ClientGetName),
                _ => Err(CmdError::SyntaxError),
            }
        }
        "HELLO" => Ok(ParsedCmd::Hello),
        // Transaction
        "MULTI" => Ok(ParsedCmd::Multi),
        "EXEC" => Ok(ParsedCmd::Exec),
        "DISCARD" => Ok(ParsedCmd::Discard),
        "UNWATCH" => Ok(ParsedCmd::Unwatch),
        "WATCH" => {
            if args.is_empty() {
                return Err(wrong_arg_count("watch"));
            }
            Ok(ParsedCmd::Watch { keys: args })
        }
        // Pub/Sub
        "PUBLISH" => {
            if args.len() != 2 {
                return Err(wrong_arg_count("publish"));
            }
            let mut iter = args.into_iter();
            let channel = iter.next().unwrap();
            let message = iter.next().unwrap();
            Ok(ParsedCmd::Publish { channel, message })
        }
        "SUBSCRIBE" => {
            if args.is_empty() {
                return Err(wrong_arg_count("subscribe"));
            }
            Ok(ParsedCmd::Subscribe { channels: args })
        }
        "UNSUBSCRIBE" => Ok(ParsedCmd::Unsubscribe { channels: args }),
        // SCAN
        "SCAN" => {
            if args.is_empty() {
                return Err(wrong_arg_count("scan"));
            }
            let cursor = args[0].parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
            let (match_pattern, count, type_filter) = parse_scan_args(&args[1..]);
            Ok(ParsedCmd::Scan { cursor, match_pattern, count, type_filter })
        }
        "SSCAN" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("sscan"));
            }
            let key = args[0].clone();
            let cursor = args[1].parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
            let (match_pattern, count, _type_filter) = parse_scan_args(&args[2..]);
            Ok(ParsedCmd::Sscan { key, cursor, match_pattern, count })
        }
        "HSCAN" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("hscan"));
            }
            let key = args[0].clone();
            let cursor = args[1].parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
            let (match_pattern, count, _type_filter) = parse_scan_args(&args[2..]);
            Ok(ParsedCmd::Hscan { key, cursor, match_pattern, count })
        }
        "ZSCAN" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("zscan"));
            }
            let key = args[0].clone();
            let cursor = args[1].parse::<u64>().map_err(|_| CmdError::InvalidInteger)?;
            let (match_pattern, count, _type_filter) = parse_scan_args(&args[2..]);
            Ok(ParsedCmd::Zscan { key, cursor, match_pattern, count })
        }
        // Geo
        "GEOADD" => {
            if args.len() < 4 || (args.len() - 1) % 3 != 0 {
                return Err(wrong_arg_count("geoadd"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let mut members = Vec::new();
            while let Some(lon_str) = iter.next() {
                let lon = lon_str.parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
                let lat = iter.next().ok_or_else(|| wrong_arg_count("geoadd"))?
                    .parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
                let member = iter.next().ok_or_else(|| wrong_arg_count("geoadd"))?;
                members.push((lon, lat, member));
            }
            Ok(ParsedCmd::GeoAdd { key, members })
        }
        "GEODIST" => {
            if args.len() < 3 || args.len() > 4 {
                return Err(wrong_arg_count("geodist"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let member1 = iter.next().unwrap();
            let member2 = iter.next().unwrap();
            let unit = iter.next().unwrap_or_else(|| "m".to_string());
            if !matches!(unit.as_str(), "m" | "km" | "mi" | "ft") {
                return Err(CmdError::SyntaxError);
            }
            Ok(ParsedCmd::GeoDist { key, member1, member2, unit })
        }
        "GEOHASH" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("geohash"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let members: Vec<String> = iter.collect();
            Ok(ParsedCmd::GeoHash { key, members })
        }
        "GEOPOS" => {
            if args.len() < 2 {
                return Err(wrong_arg_count("geopos"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let members: Vec<String> = iter.collect();
            Ok(ParsedCmd::GeoPos { key, members })
        }
        "GEORADIUS" => {
            if args.len() < 5 {
                return Err(wrong_arg_count("georadius"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let lon = iter.next().unwrap().parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
            let lat = iter.next().unwrap().parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
            let radius = iter.next().unwrap().parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
            let unit = iter.next().unwrap();
            if !matches!(unit.as_str(), "m" | "km" | "mi" | "ft") {
                return Err(CmdError::SyntaxError);
            }
            let mut withcoord = false;
            let mut withdist = false;
            let mut count = None;
            while let Some(flag) = iter.next() {
                match flag.to_uppercase().as_str() {
                    "WITHCOORD" => withcoord = true,
                    "WITHDIST" => withdist = true,
                    "COUNT" => {
                        count = Some(iter.next().ok_or_else(|| wrong_arg_count("georadius"))?
                            .parse::<u64>().map_err(|_| CmdError::InvalidInteger)?);
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
            }
            Ok(ParsedCmd::GeoRadius { key, longitude: lon, latitude: lat, radius, unit, withcoord, withdist, count })
        }
        "GEORADIUSBYMEMBER" => {
            if args.len() < 4 {
                return Err(wrong_arg_count("georadiusbymember"));
            }
            let mut iter = args.into_iter();
            let key = iter.next().unwrap();
            let member = iter.next().unwrap();
            let radius = iter.next().unwrap().parse::<f64>().map_err(|_| CmdError::InvalidInteger)?;
            let unit = iter.next().unwrap();
            if !matches!(unit.as_str(), "m" | "km" | "mi" | "ft") {
                return Err(CmdError::SyntaxError);
            }
            let mut withcoord = false;
            let mut withdist = false;
            let mut count = None;
            while let Some(flag) = iter.next() {
                match flag.to_uppercase().as_str() {
                    "WITHCOORD" => withcoord = true,
                    "WITHDIST" => withdist = true,
                    "COUNT" => {
                        count = Some(iter.next().ok_or_else(|| wrong_arg_count("georadiusbymember"))?
                            .parse::<u64>().map_err(|_| CmdError::InvalidInteger)?);
                    }
                    _ => return Err(CmdError::SyntaxError),
                }
            }
            Ok(ParsedCmd::GeoRadiusByMember { key, member, radius, unit, withcoord, withdist, count })
        }
        _ => Err(CmdError::UnknownCommand),
    }
}

// ── Helpers ──

/// Parse optional MATCH, COUNT, and TYPE from SCAN-style args.
fn parse_scan_args(args: &[String]) -> (Option<String>, u64, Option<String>) {
    let mut i = 0;
    let mut match_pattern = None;
    let mut count = 10u64;
    let mut type_filter = None;
    while i < args.len() {
        match args[i].to_uppercase().as_str() {
            "MATCH" => {
                i += 1;
                match_pattern = args.get(i).cloned();
            }
            "COUNT" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    count = s.parse().unwrap_or(10);
                }
            }
            "TYPE" => {
                i += 1;
                type_filter = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }
    (match_pattern, count, type_filter)
}
