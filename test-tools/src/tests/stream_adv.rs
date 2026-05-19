use crate::helpers::*;
use crate::RedisClient;
use mini_redis::protocol::resp::RespType;

pub async fn test_xgroup_create(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["DEL", "test_adv:s1"]).await?;
    let _ = client.cmd(&["XADD", "test_adv:s1", "1-0", "f", "v"]).await?;
    let r = client.cmd(&["XGROUP", "CREATE", "test_adv:s1", "mygroup", "0"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "XGROUP CREATE");
    Ok(())
}

pub async fn test_xgroup_destroy(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["DEL", "test_adv:s2"]).await?;
    let _ = client.cmd(&["XADD", "test_adv:s2", "1-0", "f", "v"]).await?;
    let _ = client.cmd(&["XGROUP", "CREATE", "test_adv:s2", "g2", "0"]).await?;
    let r = client.cmd(&["XGROUP", "DESTROY", "test_adv:s2", "g2"]).await?;
    crate::assert_resp!(r, int(1), "XGROUP DESTROY");
    Ok(())
}

pub async fn test_xreadgroup_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["DEL", "test_adv:s3"]).await?;
    let _ = client.cmd(&["XADD", "test_adv:s3", "1-0", "f", "v"]).await?;
    let _ = client.cmd(&["XGROUP", "CREATE", "test_adv:s3", "g3", "0"]).await?;
    let r = client.cmd(&["XREADGROUP", "GROUP", "g3", "c1", "STREAMS", "test_adv:s3", ">"]).await?;
    match &r {
        RespType::Array(Some(_)) => Ok(()),
        _ => Err(format!("XREADGROUP: expected Array, got {}", r)),
    }
}

pub async fn test_xack_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["DEL", "test_adv:s4"]).await?;
    let _ = client.cmd(&["XADD", "test_adv:s4", "1-0", "f", "v"]).await?;
    let _ = client.cmd(&["XGROUP", "CREATE", "test_adv:s4", "g4", "0"]).await?;
    let _ = client.cmd(&["XREADGROUP", "GROUP", "g4", "c1", "STREAMS", "test_adv:s4", ">"]).await?;
    let r = client.cmd(&["XACK", "test_adv:s4", "g4", "1-0"]).await?;
    crate::assert_resp!(r, int(1), "XACK");
    Ok(())
}

pub async fn test_xpending_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["DEL", "test_adv:s5"]).await?;
    let _ = client.cmd(&["XADD", "test_adv:s5", "1-0", "f", "v"]).await?;
    let _ = client.cmd(&["XGROUP", "CREATE", "test_adv:s5", "g5", "0"]).await?;
    let _ = client.cmd(&["XREADGROUP", "GROUP", "g5", "c1", "STREAMS", "test_adv:s5", ">"]).await?;
    let r = client.cmd(&["XPENDING", "test_adv:s5", "g5"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.is_empty() {
                Err("XPENDING: expected non-empty".to_string())
            } else {
                Ok(())
            }
        }
        _ => Err(format!("XPENDING: expected Array, got {}", r)),
    }
}

pub async fn test_xinfo_stream(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["DEL", "test_adv:s6"]).await?;
    let _ = client.cmd(&["XADD", "test_adv:s6", "1-0", "f", "v"]).await?;
    let _ = client.cmd(&["XGROUP", "CREATE", "test_adv:s6", "g6", "0"]).await?;
    let r = client.cmd(&["XINFO", "STREAM", "test_adv:s6"]).await?;
    crate::assert_match!(r, RespType::Array(Some(_)), "XINFO STREAM");
    Ok(())
}
