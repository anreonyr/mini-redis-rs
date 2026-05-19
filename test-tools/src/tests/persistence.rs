use crate::helpers;
use crate::RedisClient;

pub async fn test_save_basic(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["SET", "persist:test", "hello"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET should succeed");

    let resp = client.cmd(&["SAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SAVE should return OK");

    let path = std::path::Path::new("dump.db");
    if !path.exists() {
        return Err("SAVE did not create dump.db".to_string());
    }

    let _ = std::fs::remove_file("dump.db");
    Ok(())
}

pub async fn test_save_roundtrip(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["SET", "rt:string", "value1"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET string");

    let resp = client.cmd(&["RPUSH", "rt:list", "a", "b", "c"]).await?;
    crate::assert_resp!(resp, helpers::int(3), "RPUSH list");

    let resp = client.cmd(&["HSET", "rt:hash", "field1", "val1"]).await?;
    crate::assert_resp!(resp, helpers::int(1), "HSET hash");

    let resp = client.cmd(&["SADD", "rt:set", "member1"]).await?;
    crate::assert_resp!(resp, helpers::int(1), "SADD set");

    let resp = client.cmd(&["SAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SAVE");

    let path = std::path::Path::new("dump.db");
    if !path.exists() {
        return Err("SAVE did not create dump.db".to_string());
    }

    let _ = std::fs::remove_file("dump.db");
    Ok(())
}

pub async fn test_bgsave(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["SET", "bgsave:test", "world"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET");

    let resp = client.cmd(&["BGSAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "BGSAVE should return OK");

    // Wait a bit for background save to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let path = std::path::Path::new("dump.db");
    if !path.exists() {
        return Err("BGSAVE did not create dump.db".to_string());
    }

    let _ = std::fs::remove_file("dump.db");
    Ok(())
}

pub async fn test_config_get_dir(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["CONFIG", "GET", "dir"]).await?;
    crate::assert_match!(resp, mini_redis::resp::RespType::Array(Some(_)), "CONFIG GET dir should return array");
    Ok(())
}

pub async fn test_config_set_dir(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["CONFIG", "SET", "dir", "/tmp"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "CONFIG SET dir");

    let resp = client.cmd(&["CONFIG", "SET", "dir", "."]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "CONFIG SET dir back");
    Ok(())
}
