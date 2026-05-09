use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_wrongtype_get_on_list(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["RPUSH", "test_rs:wt_list", "a"]).await?;
    let r = client.cmd(&["GET", "test_rs:wt_list"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("GET on list: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_llen_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_str", "val"]).await?;
    let r = client.cmd(&["LLEN", "test_rs:wt_str"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("LLEN on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_rpush_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_rpush", "val"]).await?;
    let r = client.cmd(&["RPUSH", "test_rs:wt_rpush", "a"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("RPUSH on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_lpop_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_lpop", "val"]).await?;
    let r = client.cmd(&["LPOP", "test_rs:wt_lpop"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("LPOP on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_lrange_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_lrange", "val"]).await?;
    let r = client.cmd(&["LRANGE", "test_rs:wt_lrange", "0", "-1"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("LRANGE on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_blpop_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_blpop", "val"]).await?;
    let r = client.cmd(&["BLPOP", "test_rs:wt_blpop", "1"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("BLPOP on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_hget_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_hget", "val"]).await?;
    let r = client.cmd(&["HGET", "test_rs:wt_hget", "field"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("HGET on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_hset_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_hset", "val"]).await?;
    let r = client.cmd(&["HSET", "test_rs:wt_hset", "f", "v"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("HSET on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_sadd_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_sadd", "val"]).await?;
    let r = client.cmd(&["SADD", "test_rs:wt_sadd", "member"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("SADD on string: expected WRONGTYPE, got {}", r)),
    }
}

pub async fn test_wrongtype_zadd_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:wt_zadd", "val"]).await?;
    let r = client.cmd(&["ZADD", "test_rs:wt_zadd", "1", "member"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("ZADD on string: expected WRONGTYPE, got {}", r)),
    }
}
