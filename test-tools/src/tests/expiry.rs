use crate::helpers::*;
use crate::RedisClient;

pub async fn test_ex_actual_expiry(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:exp_ex", "val", "EX", "1"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET EX 1");
    let r = client.cmd(&["GET", "test_rs:exp_ex"]).await?;
    crate::assert_resp!(r, bulk_str("val"), "GET before expiry");
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let r = client.cmd(&["GET", "test_rs:exp_ex"]).await?;
    crate::assert_resp!(r, null_bulk(), "GET after EX expiry");
    Ok(())
}

pub async fn test_px_actual_expiry(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:exp_px", "val", "PX", "500"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET PX 500");
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    let r = client.cmd(&["GET", "test_rs:exp_px"]).await?;
    crate::assert_resp!(r, null_bulk(), "GET after PX expiry");
    Ok(())
}

pub async fn test_expiry_background_cleanup(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:exp_bg", "val", "EX", "1"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET EX 1 for bg cleanup");
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let r = client.cmd(&["GET", "test_rs:exp_bg"]).await?;
    crate::assert_resp!(r, null_bulk(), "GET after background cleanup");
    Ok(())
}

pub async fn test_expire_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "t", "v"]).await?;
    let r = client.cmd(&["EXPIRE", "t", "10"]).await?;
    crate::assert_resp!(r, int(1), "EXPIRE should return 1");
    let r = client.cmd(&["TTL", "t"]).await?;
    assert!(matches!(&r, crate::RespType::Integer(n) if *n > 0 && *n <= 10), "TTL should be between 1 and 10, got {:?}", r);
    Ok(())
}

pub async fn test_expire_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["EXPIRE", "no", "10"]).await?;
    crate::assert_resp!(r, int(0), "EXPIRE nonexistent key should return 0");
    Ok(())
}

pub async fn test_ttl_with_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "t2", "v", "EX", "100"]).await?;
    let r = client.cmd(&["TTL", "t2"]).await?;
    assert!(matches!(&r, crate::RespType::Integer(n) if *n > 0 && *n <= 100), "TTL with expiry should be positive, got {:?}", r);
    Ok(())
}

pub async fn test_ttl_no_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "t3", "v"]).await?;
    let r = client.cmd(&["TTL", "t3"]).await?;
    crate::assert_resp!(r, int(-1), "TTL without expiry should return -1");
    Ok(())
}

pub async fn test_ttl_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["TTL", "no"]).await?;
    crate::assert_resp!(r, int(-2), "TTL nonexistent key should return -2");
    Ok(())
}

pub async fn test_persist_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "t4", "v", "EX", "100"]).await?;
    let r = client.cmd(&["PERSIST", "t4"]).await?;
    crate::assert_resp!(r, int(1), "PERSIST should return 1");
    let r = client.cmd(&["TTL", "t4"]).await?;
    crate::assert_resp!(r, int(-1), "TTL after PERSIST should be -1");
    Ok(())
}

pub async fn test_persist_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PERSIST", "no"]).await?;
    crate::assert_resp!(r, int(0), "PERSIST nonexistent key should return 0");
    Ok(())
}

pub async fn test_persist_no_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "t5", "v"]).await?;
    let r = client.cmd(&["PERSIST", "t5"]).await?;
    crate::assert_resp!(r, int(0), "PERSIST key without expiry should return 0");
    Ok(())
}
