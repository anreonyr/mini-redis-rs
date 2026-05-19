use crate::helpers;
use crate::RedisClient;

const DUMP_FILE: &str = "dump.db";

async fn clean_dump() {
    let _ = tokio::fs::remove_file(DUMP_FILE).await;
}

pub async fn test_save_basic(client: &mut RedisClient) -> Result<(), String> {
    clean_dump().await;

    let resp = client.cmd(&["SET", "persist:test", "hello"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET should succeed");

    let resp = client.cmd(&["SAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SAVE should return OK");

    if !std::path::Path::new(DUMP_FILE).exists() {
        return Err("SAVE did not create dump.db".to_string());
    }

    clean_dump().await;
    Ok(())
}

pub async fn test_save_multiple_types(client: &mut RedisClient) -> Result<(), String> {
    clean_dump().await;

    let resp = client.cmd(&["SET", "mt:string", "value1"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET string");

    let resp = client.cmd(&["RPUSH", "mt:list", "a", "b", "c"]).await?;
    crate::assert_resp!(resp, helpers::int(3), "RPUSH list");

    let resp = client.cmd(&["HSET", "mt:hash", "field1", "val1"]).await?;
    crate::assert_resp!(resp, helpers::int(1), "HSET hash");

    let resp = client.cmd(&["SADD", "mt:set", "member1"]).await?;
    crate::assert_resp!(resp, helpers::int(1), "SADD set");

    let resp = client.cmd(&["SAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SAVE");

    if !std::path::Path::new(DUMP_FILE).exists() {
        return Err("SAVE did not create dump.db".to_string());
    }

    clean_dump().await;
    Ok(())
}

pub async fn test_bgsave(client: &mut RedisClient) -> Result<(), String> {
    clean_dump().await;

    let resp = client.cmd(&["SET", "bgsave:test", "world"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET");

    let resp = client.cmd(&["BGSAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "BGSAVE should return OK");

    // Poll for the dump file (up to 5s) instead of fixed sleep
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if std::path::Path::new(DUMP_FILE).exists() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("BGSAVE did not create dump.db within 5s".to_string());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    clean_dump().await;
    Ok(())
}

pub async fn test_config_set_dir(client: &mut RedisClient) -> Result<(), String> {
    // Save original value
    let resp = client.cmd(&["CONFIG", "GET", "dir"]).await?;
    let original = match &resp {
        mini_redis::protocol::resp::RespType::Array(Some(items)) if items.len() == 2 => {
            if let mini_redis::protocol::resp::RespType::BulkString(Some(bytes)) = &items[1] {
                String::from_utf8_lossy(bytes).to_string()
            } else {
                return Err("CONFIG GET dir: second element is not BulkString".to_string());
            }
        }
        _ => return Err(format!("CONFIG GET dir: unexpected response: {}", resp)),
    };

    // SET to /tmp, verify with GET
    let resp = client.cmd(&["CONFIG", "SET", "dir", "/tmp"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "CONFIG SET dir");

    let resp = client.cmd(&["CONFIG", "GET", "dir"]).await?;
    crate::assert_resp!(
        resp,
        helpers::arr_of_bulks(&["dir", "/tmp"]),
        "CONFIG GET dir after SET /tmp"
    );

    // Restore original
    let resp = client.cmd(&["CONFIG", "SET", "dir", &original]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "CONFIG SET dir restore");
    Ok(())
}
