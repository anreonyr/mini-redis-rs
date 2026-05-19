use crate::helpers::*;
use crate::RedisClient;
use mini_redis::protocol::resp::RespType;

pub async fn test_command_plain(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["COMMAND"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() < 10 {
                return Err(format!("COMMAND: expected at least 10 items, got {}", items.len()));
            }
            let names: Vec<String> = items.iter().filter_map(|i| {
                if let RespType::BulkString(Some(b)) = i {
                    Some(String::from_utf8_lossy(b).to_string())
                } else {
                    None
                }
            }).collect();
            for required in &["PING", "GET", "SET", "COMMAND", "FLUSHDB"] {
                if !names.iter().any(|n| n.eq_ignore_ascii_case(required)) {
                    return Err(format!("COMMAND: missing required command {required}"));
                }
            }
            Ok(())
        }
        _ => Err(format!("COMMAND: expected Array, got {}", r)),
    }
}

pub async fn test_command_info_all(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["COMMAND", "INFO"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            for item in items {
                match item {
                    RespType::Array(Some(fields)) => {
                        if fields.len() != 6 {
                            return Err(format!("COMMAND INFO: expected 6 fields, got {}: {}",
                                fields.len(), RespType::Array(Some(fields.clone())).to_string()));
                        }
                    }
                    _ => return Err(format!("COMMAND INFO: expected Array of Arrays, got {}", r)),
                }
            }
            if items.len() < 10 {
                return Err(format!("COMMAND INFO: expected >= 10 entries, got {}", items.len()));
            }
            Ok(())
        }
        _ => Err(format!("COMMAND INFO: expected Array, got {}", r)),
    }
}

pub async fn test_command_info_specific(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["COMMAND", "INFO", "PING"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 1 => {
            match &items[0] {
                RespType::Array(Some(fields)) if fields.len() == 6 => {
                    if let RespType::BulkString(Some(name)) = &fields[0] {
                        if String::from_utf8_lossy(name).eq_ignore_ascii_case("PING") {
                            return Ok(());
                        }
                    }
                    Err(format!("COMMAND INFO PING: unexpected format: {}", r))
                }
                _ => Err(format!("COMMAND INFO PING: expected inner Array of 6, got {}", r)),
            }
        }
        RespType::Array(Some(items)) => Err(format!(
            "COMMAND INFO PING: expected 1-element array, got {} elements: {}", items.len(), r)),
        _ => Err(format!("COMMAND INFO PING: expected Array, got {}", r)),
    }
}

pub async fn test_command_info_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["COMMAND", "INFO", "FOOBAR_NONEXIST"]).await?;
    crate::assert_resp!(r, null_array(), "COMMAND INFO nonexistent");
    Ok(())
}

pub async fn test_command_unknown_subcommand(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["COMMAND", "FOO"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() < 10 {
                return Err(format!("COMMAND FOO: expected at least 10 items, got {}", items.len()));
            }
            Ok(())
        }
        _ => Err(format!("COMMAND FOO: expected Array of names, got {}", r)),
    }
}
