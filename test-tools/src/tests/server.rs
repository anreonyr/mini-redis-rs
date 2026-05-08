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
