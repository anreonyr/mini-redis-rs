use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_scan_empty(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SCAN", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => Ok(()),
        _ => Err(format!("SCAN empty: expected Array(2), got {}", r)),
    }
}

pub async fn test_scan_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "scan:a", "1"]).await?;
    let _ = client.cmd(&["SET", "scan:b", "2"]).await?;
    let _ = client.cmd(&["SET", "scan:c", "3"]).await?;
    let r = client.cmd(&["SCAN", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            match &items[1] {
                RespType::Array(Some(results)) => {
                    if results.is_empty() {
                        return Err("SCAN: expected some results".to_string());
                    }
                    Ok(())
                }
                _ => Err("SCAN: expected Array of results".to_string()),
            }
        }
        _ => Err(format!("SCAN: expected Array(2), got {}", r)),
    }
}

pub async fn test_scan_match(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "scanmatch:x", "1"]).await?;
    let _ = client.cmd(&["SET", "other:y", "2"]).await?;
    let r = client.cmd(&["SCAN", "0", "MATCH", "scanmatch:*"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if let RespType::Array(Some(results)) = &items[1] {
                if results.is_empty() {
                    return Err("SCAN MATCH: expected results".to_string());
                }
                for res in results {
                    if let RespType::BulkString(Some(k)) = res {
                        let key = String::from_utf8_lossy(k);
                        if !key.starts_with("scanmatch:") {
                            return Err(format!(
                                "SCAN MATCH: key '{}' does not match pattern",
                                key
                            ));
                        }
                    }
                }
            }
            Ok(())
        }
        _ => Err(format!("SCAN MATCH: expected Array, got {}", r)),
    }
}

pub async fn test_scan_count(client: &mut RedisClient) -> Result<(), String> {
    // Set up 20 keys
    for i in 0..20 {
        let key = format!("scancount:{}", i);
        let _ = client.cmd(&["SET", &key, "v"]).await?;
    }
    let r = client.cmd(&["SCAN", "0", "COUNT", "5"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if let RespType::Array(Some(results)) = &items[1] {
                if results.len() > 10 {
                    return Err(format!(
                        "SCAN COUNT: expected <=10 results, got {}",
                        results.len()
                    ));
                }
            }
            Ok(())
        }
        _ => Err(format!("SCAN COUNT: expected Array, got {}", r)),
    }
}

pub async fn test_scan_full_cursor(client: &mut RedisClient) -> Result<(), String> {
    // Set up 30 keys and iterate through them all
    for i in 0..30 {
        let key = format!("scancursor:{}", i);
        let _ = client.cmd(&["SET", &key, "v"]).await?;
    }
    let mut cursor = "0".to_string();
    let mut all_keys = Vec::new();
    loop {
        let r = client.cmd(&["SCAN", &cursor, "COUNT", "7"]).await?;
        match &r {
            RespType::Array(Some(items)) if items.len() == 2 => {
                // Get next cursor
                if let RespType::BulkString(Some(next)) = &items[0] {
                    cursor = String::from_utf8_lossy(next).to_string();
                }
                // Get results
                if let RespType::Array(Some(results)) = &items[1] {
                    for res in results {
                        if let RespType::BulkString(Some(k)) = res {
                            all_keys.push(String::from_utf8_lossy(k).to_string());
                        }
                    }
                }
            }
            _ => return Err(format!("SCAN full cursor: unexpected response: {}", r)),
        }
        if cursor == "0" {
            break;
        }
    }
    // We should have collected all scancursor:* keys
    let count = all_keys
        .iter()
        .filter(|k| k.starts_with("scancursor:"))
        .count();
    if count != 30 {
        return Err(format!(
            "SCAN full cursor: expected 30 scancursor: keys, got {}",
            count
        ));
    }
    Ok(())
}

pub async fn test_sscan_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "myset", "a", "b", "c"]).await?;
    let r = client.cmd(&["SSCAN", "myset", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            if let RespType::Array(Some(results)) = &items[1] {
                if results.is_empty() {
                    return Err("SSCAN: expected some results".to_string());
                }
            }
            Ok(())
        }
        _ => Err(format!("SSCAN: expected Array(2), got {}", r)),
    }
}

pub async fn test_sscan_match(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SADD", "sscan_set", "aa", "ab", "bb", "cc"]).await?;
    let r = client.cmd(&["SSCAN", "sscan_set", "0", "MATCH", "a*"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if let RespType::Array(Some(results)) = &items[1] {
                if results.is_empty() {
                    return Err("SSCAN MATCH: expected some results".to_string());
                }
            }
            Ok(())
        }
        _ => Err(format!("SSCAN MATCH: expected Array, got {}", r)),
    }
}

pub async fn test_hscan_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client
        .cmd(&["HSET", "myhash", "field1", "val1", "field2", "val2"])
        .await?;
    let r = client.cmd(&["HSCAN", "myhash", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            if let RespType::Array(Some(results)) = &items[1] {
                // Each field is returned as key-value pair, so should have 4 items for 2 fields
                if results.len() != 4 {
                    return Err(format!(
                        "HSCAN: expected 4 results (2 field-value pairs), got {}",
                        results.len()
                    ));
                }
            }
            Ok(())
        }
        _ => Err(format!("HSCAN: expected Array(2), got {}", r)),
    }
}

pub async fn test_hscan_match(client: &mut RedisClient) -> Result<(), String> {
    let _ = client
        .cmd(&[
            "HSET", "hscan_hash", "alpha", "1", "beta", "2", "gamma", "3",
        ])
        .await?;
    let r = client.cmd(&["HSCAN", "hscan_hash", "0", "MATCH", "a*"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if let RespType::Array(Some(results)) = &items[1] {
                if results.is_empty() {
                    return Err("HSCAN MATCH: expected some results".to_string());
                }
            }
            Ok(())
        }
        _ => Err(format!("HSCAN MATCH: expected Array, got {}", r)),
    }
}

pub async fn test_zscan_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client
        .cmd(&["ZADD", "myzset", "1", "one", "2", "two", "3", "three"])
        .await?;
    let r = client.cmd(&["ZSCAN", "myzset", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            if let RespType::Array(Some(results)) = &items[1] {
                // Each member returns (member, score), so 3 members = 6 items
                if results.len() != 6 {
                    return Err(format!(
                        "ZSCAN: expected 6 results (3 member-score pairs), got {}",
                        results.len()
                    ));
                }
            }
            Ok(())
        }
        _ => Err(format!("ZSCAN: expected Array(2), got {}", r)),
    }
}

pub async fn test_zscan_match(client: &mut RedisClient) -> Result<(), String> {
    let _ = client
        .cmd(&["ZADD", "zscan_zset", "10", "apple", "20", "banana", "30", "avocado"])
        .await?;
    let r = client.cmd(&["ZSCAN", "zscan_zset", "0", "MATCH", "a*"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if let RespType::Array(Some(results)) = &items[1] {
                if results.is_empty() {
                    return Err("ZSCAN MATCH: expected some results".to_string());
                }
            }
            Ok(())
        }
        _ => Err(format!("ZSCAN MATCH: expected Array, got {}", r)),
    }
}

pub async fn test_scan_non_existent_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SSCAN", "nonexistent", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            if let RespType::BulkString(Some(cursor)) = &items[0] {
                if cursor.as_ref() != b"0" {
                    return Err(format!(
                        "SSCAN nonexistent: expected cursor 0, got {}",
                        String::from_utf8_lossy(cursor)
                    ));
                }
            }
            if let RespType::Array(Some(results)) = &items[1] {
                if !results.is_empty() {
                    return Err("SSCAN nonexistent: expected empty results".to_string());
                }
            }
            Ok(())
        }
        _ => Err(format!(
            "SSCAN nonexistent: expected Array(2), got {}",
            r
        )),
    }
}

pub async fn test_wrongtype_scan_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "mystr", "hello"]).await?;
    let r = client.cmd(&["SSCAN", "mystr", "0"]).await?;
    match &r {
        RespType::Error(msg) => {
            if msg == "WRONGTYPE" {
                Ok(())
            } else {
                Err(format!(
                    "SSCAN on string: expected WRONGTYPE, got {}",
                    msg
                ))
            }
        }
        _ => Err(format!(
            "SSCAN on string: expected Error, got {}",
            r
        )),
    }
}
