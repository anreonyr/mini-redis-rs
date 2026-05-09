use crate::resp::RespType;

pub fn handle_hset(key: &str, fields: &[(String, String)]) -> RespType {
    let _ = (key, fields);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hget(key: &str, field: &str) -> RespType {
    let _ = (key, field);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hdel(key: &str, fields: &[String]) -> RespType {
    let _ = (key, fields);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hgetall(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hexists(key: &str, field: &str) -> RespType {
    let _ = (key, field);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hlen(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hkeys(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_hvals(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}
