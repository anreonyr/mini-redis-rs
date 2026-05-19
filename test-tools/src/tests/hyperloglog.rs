use crate::helpers::*;
use crate::RedisClient;

pub async fn test_pfadd_basic(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PFADD", "hll:test", "a", "b", "c"]).await?;
    crate::assert_resp!(r, int(1), "PFADD should return 1 for new elements");
    Ok(())
}

pub async fn test_pfadd_duplicate(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["PFADD", "hll:dup", "a"]).await?;
    let r = client.cmd(&["PFADD", "hll:dup", "a"]).await?;
    crate::assert_resp!(r, int(0), "PFADD duplicate should return 0");
    Ok(())
}

pub async fn test_pfcount_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["PFADD", "hll:count", "x", "y", "z"]).await?;
    let r = client.cmd(&["PFCOUNT", "hll:count"]).await?;
    crate::assert_resp!(r, int(3), "PFCOUNT should be 3");
    Ok(())
}

pub async fn test_pfcount_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PFCOUNT", "hll:nonexist"]).await?;
    crate::assert_resp!(r, int(0), "PFCOUNT nonexistent should be 0");
    Ok(())
}

pub async fn test_pfmerge_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["PFADD", "hll:src1", "a", "b", "c"]).await?;
    let _ = client.cmd(&["PFADD", "hll:src2", "d", "e", "f"]).await?;
    let r = client.cmd(&["PFMERGE", "hll:dest", "hll:src1", "hll:src2"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "PFMERGE should return OK");
    let r = client.cmd(&["PFCOUNT", "hll:dest"]).await?;
    crate::assert_resp!(r, int(6), "PFCOUNT of merged HLL should be 6");
    Ok(())
}

pub async fn test_pfadd_wrongtype(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "hll:wt", "stringvalue"]).await?;
    let r = client.cmd(&["PFADD", "hll:wt", "a"]).await?;
    assert!(
        matches!(&r, mini_redis::resp::RespType::Error(msg) if msg.contains("WRONGTYPE")),
        "PFADD on string should return WRONGTYPE, got: {}",
        r
    );
    Ok(())
}

pub async fn test_pfcount_multiple_keys(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["PFADD", "hll:mk1", "a", "b"]).await?;
    let _ = client.cmd(&["PFADD", "hll:mk2", "c", "d"]).await?;
    let r = client.cmd(&["PFCOUNT", "hll:mk1", "hll:mk2"]).await?;
    crate::assert_resp!(r, int(4), "PFCOUNT of union should be 4");
    Ok(())
}
