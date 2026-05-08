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
