use crate::resp::RespType;

pub fn handle_sadd(key: &str, members: &[String]) -> RespType {
    let _ = (key, members);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_smembers(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_sismember(key: &str, member: &str) -> RespType {
    let _ = (key, member);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_srem(key: &str, members: &[String]) -> RespType {
    let _ = (key, members);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_scard(key: &str) -> RespType {
    let _ = key;
    RespType::Error("ERR not implemented".to_string())
}
