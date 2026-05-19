use crate::helpers::*;
use crate::RedisClient;
use mini_redis::protocol::resp::RespType;

pub async fn test_blpop_immediate(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["RPUSH", "test_rs:blpop_imm", "val"]).await?;
    let now = tokio::time::Instant::now();
    let r = client.cmd(&["BLPOP", "test_rs:blpop_imm", "0"]).await?;
    let elapsed = now.elapsed();
    if elapsed.as_millis() > 100 {
        return Err(format!("BLPOP immediate: took {}ms, expected < 100ms", elapsed.as_millis()));
    }
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => Ok(()),
        _ => Err(format!("BLPOP immediate: expected Array of 2, got {}", r)),
    }
}

pub async fn test_blpop_timeout(client: &mut RedisClient) -> Result<(), String> {
    let now = tokio::time::Instant::now();
    let r = client.cmd(&["BLPOP", "test_rs:blpop_empty", "1"]).await?;
    let elapsed = now.elapsed();
    if elapsed.as_millis() < 800 {
        return Err(format!("BLPOP timeout: took {}ms, expected >= 800ms", elapsed.as_millis()));
    }
    crate::assert_resp!(r, null_array(), "BLPOP timeout");
    Ok(())
}

pub async fn test_blpop_multi_key(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["RPUSH", "test_rs:blpop_multi", "winner"]).await?;
    let r = client.cmd(&["BLPOP", "test_rs:blpop_empty", "test_rs:blpop_multi", "1"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            if let RespType::BulkString(Some(key)) = &items[0] {
                if String::from_utf8_lossy(key) == "test_rs:blpop_multi" {
                    return Ok(());
                }
            }
            Err(format!("BLPOP multi-key: unexpected format: {}", r))
        }
        _ => Err(format!("BLPOP multi-key: expected Array of 2, got {}", r)),
    }
}

pub async fn test_blpop_wakeup(client_b: &mut RedisClient) -> Result<(), String> {
    let mut client_a = RedisClient::connect("127.0.0.1:6379").await?;
    let handle_a = tokio::spawn(async move {
        let now = tokio::time::Instant::now();
        let r = client_a.cmd(&["BLPOP", "test_rs:blpop_wakeup", "5"]).await;
        (now.elapsed(), r)
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let r = client_b.cmd(&["RPUSH", "test_rs:blpop_wakeup", "wakeup"]).await?;
    crate::assert_resp!(r, int(1), "RPUSH wakeup");
    let (elapsed, result) = handle_a.await.map_err(|e| format!("join error: {}", e))?;
    if elapsed.as_millis() > 3000 {
        return Err(format!("BLPOP wakeup: took {}ms, expected < 3000ms", elapsed.as_millis()));
    }
    match &result {
        Ok(RespType::Array(Some(items))) if items.len() == 2 => Ok(()),
        Ok(other) => Err(format!("BLPOP wakeup: expected Array of 2, got {}", other)),
        Err(e) => Err(format!("BLPOP wakeup: client_a error: {}", e)),
    }
}

pub async fn test_blpop_zero_timeout_with_data(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["RPUSH", "test_rs:blpop_zero", "val"]).await?;
    let now = tokio::time::Instant::now();
    let r = client.cmd(&["BLPOP", "test_rs:blpop_zero", "0"]).await?;
    let elapsed = now.elapsed();
    if elapsed.as_millis() > 100 {
        return Err(format!("BLPOP 0 timeout with data: took {}ms, expected < 100ms", elapsed.as_millis()));
    }
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => Ok(()),
        _ => Err(format!("BLPOP 0 timeout with data: expected Array of 2, got {}", r)),
    }
}
