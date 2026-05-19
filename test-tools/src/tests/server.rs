use crate::helpers::*;
use crate::RedisClient;

pub async fn test_flushdb(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:flush_k", "v"]).await?;
    let r = client.cmd(&["FLUSHDB"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "FLUSHDB response");
    let r = client.cmd(&["GET", "test_rs:flush_k"]).await?;
    crate::assert_resp!(r, null_bulk(), "GET after FLUSHDB");
    Ok(())
}

pub async fn test_info(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["INFO"]).await?;
    assert!(matches!(&r, mini_redis::protocol::resp::RespType::BulkString(Some(_))), "INFO response");
    Ok(())
}

pub async fn test_config_get_dir(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["CONFIG", "GET", "dir"]).await?;
    assert!(matches!(&r, mini_redis::protocol::resp::RespType::Array(Some(v)) if v.len() == 2), "CONFIG GET dir");
    Ok(())
}

pub async fn test_config_get_unknown(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["CONFIG", "GET", "unknown_param"]).await?;
    crate::assert_resp!(r, empty_array(), "CONFIG GET unknown");
    Ok(())
}

pub async fn test_time(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["TIME"]).await?;
    match &r {
        mini_redis::protocol::resp::RespType::Array(Some(items)) if items.len() == 2 => {
            // Both elements should be BulkString with numeric values
            for item in items {
                match item {
                    mini_redis::protocol::resp::RespType::BulkString(Some(b)) => {
                        let s = String::from_utf8_lossy(b);
                        s.parse::<u64>().map_err(|_| format!("TIME: non-numeric value: {}", s))?;
                    }
                    _ => return Err(format!("TIME: expected BulkString elements, got {}", item)),
                }
            }
            Ok(())
        }
        _ => Err(format!("TIME: expected Array of 2, got {}", r)),
    }
}
