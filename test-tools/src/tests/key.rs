use crate::helpers;
use crate::RedisClient;

pub async fn test_del_single(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "k1", "v1"]).await?;
    let r = client.cmd(&["DEL", "k1"]).await?;
    crate::assert_resp!(r, helpers::int(1), "DEL single key");
    let r = client.cmd(&["GET", "k1"]).await?;
    crate::assert_resp!(r, helpers::null_bulk(), "key should be gone");
    Ok(())
}

pub async fn test_del_multiple(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "a", "1"]).await?;
    client.cmd(&["SET", "b", "2"]).await?;
    client.cmd(&["SET", "c", "3"]).await?;
    let r = client.cmd(&["DEL", "a", "b", "d"]).await?;
    crate::assert_resp!(r, helpers::int(2), "DEL multiple keys");
    Ok(())
}

pub async fn test_del_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["DEL", "no"]).await?;
    crate::assert_resp!(r, helpers::int(0), "DEL nonexistent key");
    Ok(())
}

pub async fn test_exists_single(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "k1", "v1"]).await?;
    let r = client.cmd(&["EXISTS", "k1"]).await?;
    crate::assert_resp!(r, helpers::int(1), "EXISTS single key");
    Ok(())
}

pub async fn test_exists_multiple(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["FLUSHDB"]).await?;
    client.cmd(&["SET", "a", "1"]).await?;
    client.cmd(&["SET", "b", "2"]).await?;
    let r = client.cmd(&["EXISTS", "a", "b", "c"]).await?;
    crate::assert_resp!(r, helpers::int(2), "EXISTS multiple keys");
    Ok(())
}

pub async fn test_exists_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["EXISTS", "no"]).await?;
    crate::assert_resp!(r, helpers::int(0), "EXISTS nonexistent key");
    Ok(())
}

pub async fn test_type_string(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "k", "v"]).await?;
    let r = client.cmd(&["TYPE", "k"]).await?;
    crate::assert_resp!(r, helpers::simple_str("string"), "TYPE string");
    Ok(())
}

pub async fn test_type_list(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "mylist", "a"]).await?;
    let r = client.cmd(&["TYPE", "mylist"]).await?;
    crate::assert_resp!(r, helpers::simple_str("list"), "TYPE list");
    Ok(())
}

pub async fn test_type_none(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["TYPE", "no"]).await?;
    crate::assert_resp!(r, helpers::simple_str("none"), "TYPE none");
    Ok(())
}

pub async fn test_keys_pattern(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "hello", "1"]).await?;
    client.cmd(&["SET", "hallo", "2"]).await?;
    client.cmd(&["SET", "hxllo", "3"]).await?;
    let r = client.cmd(&["KEYS", "h*llo"]).await?;
    // Should match hello and hallo and hxllo
    assert!(matches!(&r, crate::RespType::Array(Some(v)) if v.len() == 3), "KEYS h*llo should match 3 keys, got {:?}", r);
    Ok(())
}

pub async fn test_keys_nomatch(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["KEYS", "zzz"]).await?;
    crate::assert_resp!(r, helpers::empty_array(), "KEYS no match");
    Ok(())
}

pub async fn test_dbsize(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["FLUSHDB"]).await?;
    client.cmd(&["SET", "a", "1"]).await?;
    client.cmd(&["SET", "b", "2"]).await?;
    let r = client.cmd(&["DBSIZE"]).await?;
    crate::assert_resp!(r, helpers::int(2), "DBSIZE should be 2");
    Ok(())
}

pub async fn test_rename(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "old", "value"]).await?;
    let r = client.cmd(&["RENAME", "old", "new"]).await?;
    crate::assert_resp!(r, helpers::simple_str("OK"), "RENAME");
    let r = client.cmd(&["GET", "old"]).await?;
    crate::assert_resp!(r, helpers::null_bulk(), "old key should be gone");
    let r = client.cmd(&["GET", "new"]).await?;
    crate::assert_resp!(r, helpers::bulk_str("value"), "new key should have value");
    Ok(())
}

pub async fn test_renamenx(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "src", "val"]).await?;
    client.cmd(&["SET", "dst", "other"]).await?;
    let r = client.cmd(&["RENAMENX", "src", "dst"]).await?;
    crate::assert_resp!(r, helpers::int(0), "RENAMENX should fail when dst exists");
    let r = client.cmd(&["RENAMENX", "src", "free"]).await?;
    crate::assert_resp!(r, helpers::int(1), "RENAMENX should succeed");
    Ok(())
}

pub async fn test_randomkey(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["FLUSHDB"]).await?;
    client.cmd(&["SET", "k1", "v1"]).await?;
    let r = client.cmd(&["RANDOMKEY"]).await?;
    crate::assert_resp!(r, helpers::bulk_str("k1"), "RANDOMKEY");
    Ok(())
}

pub async fn test_touch_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:touch_k", "v"]).await?;
    let r = client.cmd(&["TOUCH", "test_rs:touch_k"]).await?;
    crate::assert_resp!(r, helpers::int(1), "TOUCH single key");
    Ok(())
}

pub async fn test_touch_multiple(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["SET", "test_rs:ta", "1"]).await?;
    client.cmd(&["SET", "test_rs:tb", "2"]).await?;
    let r = client.cmd(&["TOUCH", "test_rs:ta", "test_rs:tb", "test_rs:nonexist"]).await?;
    crate::assert_resp!(r, helpers::int(2), "TOUCH multiple keys (2 exist)");
    Ok(())
}
