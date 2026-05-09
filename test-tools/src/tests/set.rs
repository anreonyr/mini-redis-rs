use crate::helpers::*;
use crate::RedisClient;

pub async fn test_sadd_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SADD", "test_rs:s", "a", "b", "c"]).await?;
    crate::assert_resp!(r, int(3), "SADD new key");
    Ok(())
}

pub async fn test_sadd_existing_members(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s2", "a", "b"]).await?;
    let r = client.cmd(&["SADD", "test_rs:s2", "b", "c"]).await?;
    // b already exists, only c is new
    crate::assert_resp!(r, int(1), "SADD existing members");
    Ok(())
}

pub async fn test_sadd_duplicate(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s3", "x"]).await?;
    let r = client.cmd(&["SADD", "test_rs:s3", "x"]).await?;
    crate::assert_resp!(r, int(0), "SADD duplicate returns 0");
    Ok(())
}

pub async fn test_smembers(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s4", "a", "b", "c"]).await?;
    let r = client.cmd(&["SMEMBERS", "test_rs:s4"]).await?;
    match &r {
        mini_redis::resp::RespType::Array(Some(items)) => {
            if items.len() == 3 {
                Ok(())
            } else {
                Err(format!("SMEMBERS: expected 3 members, got {}", items.len()))
            }
        }
        _ => Err(format!("SMEMBERS: expected Array, got {}", r)),
    }
}

pub async fn test_smembers_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SMEMBERS", "test_rs:nosuchset"]).await?;
    crate::assert_resp!(r, empty_array(), "SMEMBERS empty key");
    Ok(())
}

pub async fn test_sismember_true(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s5", "member1"]).await?;
    let r = client.cmd(&["SISMEMBER", "test_rs:s5", "member1"]).await?;
    crate::assert_resp!(r, int(1), "SISMEMBER true");
    Ok(())
}

pub async fn test_sismember_false(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s6", "a"]).await?;
    let r = client.cmd(&["SISMEMBER", "test_rs:s6", "b"]).await?;
    crate::assert_resp!(r, int(0), "SISMEMBER false");
    Ok(())
}

pub async fn test_sismember_nonexistent_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SISMEMBER", "test_rs:nokey", "x"]).await?;
    crate::assert_resp!(r, int(0), "SISMEMBER nonexistent key");
    Ok(())
}

pub async fn test_srem_single(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s7", "a", "b", "c"]).await?;
    let r = client.cmd(&["SREM", "test_rs:s7", "a"]).await?;
    crate::assert_resp!(r, int(1), "SREM single member");
    Ok(())
}

pub async fn test_srem_multiple(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s8", "x", "y", "z"]).await?;
    let r = client.cmd(&["SREM", "test_rs:s8", "x", "y"]).await?;
    crate::assert_resp!(r, int(2), "SREM multiple members");
    Ok(())
}

pub async fn test_srem_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SREM", "test_rs:nokey", "x"]).await?;
    crate::assert_resp!(r, int(0), "SREM nonexistent key");
    Ok(())
}

pub async fn test_scard(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "test_rs:s9", "a", "b", "c"]).await?;
    let r = client.cmd(&["SCARD", "test_rs:s9"]).await?;
    crate::assert_resp!(r, int(3), "SCARD");
    Ok(())
}

pub async fn test_scard_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SCARD", "test_rs:nokey"]).await?;
    crate::assert_resp!(r, int(0), "SCARD empty key");
    Ok(())
}
