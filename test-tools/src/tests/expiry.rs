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

pub async fn test_pexpire(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:pexp", "v"]).await?;
    let r = client.cmd(&["PEXPIRE", "test_rs:pexp", "50000"]).await?;
    crate::assert_resp!(r, int(1), "PEXPIRE should return 1");
    let r = client.cmd(&["PTTL", "test_rs:pexp"]).await?;
    assert!(matches!(&r, crate::RespType::Integer(n) if *n > 0 && *n <= 50000), "PTTL should be between 1 and 50000, got {:?}", r);
    Ok(())
}

pub async fn test_pexpire_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PEXPIRE", "test_rs:nokey", "10000"]).await?;
    crate::assert_resp!(r, int(0), "PEXPIRE nonexistent key");
    Ok(())
}

pub async fn test_pttl_with_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:pttl_k", "v", "PX", "50000"]).await?;
    let r = client.cmd(&["PTTL", "test_rs:pttl_k"]).await?;
    assert!(matches!(&r, crate::RespType::Integer(n) if *n > 0 && *n <= 50000), "PTTL with expiry, got {:?}", r);
    Ok(())
}

pub async fn test_pttl_no_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:pttl_no", "v"]).await?;
    let r = client.cmd(&["PTTL", "test_rs:pttl_no"]).await?;
    crate::assert_resp!(r, int(-1), "PTTL no expiry should return -1");
    Ok(())
}

pub async fn test_pttl_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PTTL", "test_rs:nokey_pttl"]).await?;
    crate::assert_resp!(r, int(-2), "PTTL nonexistent should return -2");
    Ok(())
}

pub async fn test_expireat_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:eat", "v"]).await?;
    let r = client.cmd(&["EXPIREAT", "test_rs:eat", "9999999999"]).await?;
    crate::assert_resp!(r, int(1), "EXPIREAT should return 1");
    let r = client.cmd(&["EXPIRETIME", "test_rs:eat"]).await?;
    assert!(matches!(&r, crate::RespType::Integer(n) if *n > 0), "EXPIRETIME should be positive, got {:?}", r);
    Ok(())
}

pub async fn test_expireat_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["EXPIREAT", "test_rs:nokey_eat", "9999999999"]).await?;
    crate::assert_resp!(r, int(0), "EXPIREAT nonexistent key");
    Ok(())
}

pub async fn test_expiretime_no_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:et_noexp", "v"]).await?;
    let r = client.cmd(&["EXPIRETIME", "test_rs:et_noexp"]).await?;
    crate::assert_resp!(r, int(-1), "EXPIRETIME without expiry should return -1");
    Ok(())
}

pub async fn test_expiretime_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["EXPIRETIME", "test_rs:nokey_et"]).await?;
    crate::assert_resp!(r, int(-2), "EXPIRETIME nonexistent should return -2");
    Ok(())
}

pub async fn test_pexpireat_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:peat", "v"]).await?;
    let r = client.cmd(&["PEXPIREAT", "test_rs:peat", "9999999999000"]).await?;
    crate::assert_resp!(r, int(1), "PEXPIREAT should return 1");
    let r = client.cmd(&["PEXPIRETIME", "test_rs:peat"]).await?;
    assert!(matches!(&r, crate::RespType::Integer(n) if *n > 0), "PEXPIRETIME should be positive, got {:?}", r);
    Ok(())
}

pub async fn test_pexpiretime_no_expiry(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:pet_noexp", "v"]).await?;
    let r = client.cmd(&["PEXPIRETIME", "test_rs:pet_noexp"]).await?;
    crate::assert_resp!(r, int(-1), "PEXPIRETIME without expiry");
    Ok(())
}

pub async fn test_pexpiretime_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PEXPIRETIME", "test_rs:nokey_pet"]).await?;
    crate::assert_resp!(r, int(-2), "PEXPIRETIME nonexistent");
    Ok(())
}
