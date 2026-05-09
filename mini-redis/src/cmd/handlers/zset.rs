use crate::resp::RespType;

pub fn handle_zadd(key: &str, members: &[(i64, String)]) -> RespType {
    let _ = (key, members);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_zrange(key: &str, start: i64, stop: i64, withscores: bool) -> RespType {
    let _ = (key, start, stop, withscores);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_zrank(key: &str, member: &str) -> RespType {
    let _ = (key, member);
    RespType::Error("ERR not implemented".to_string())
}

pub fn handle_zscore(key: &str, member: &str) -> RespType {
    let _ = (key, member);
    RespType::Error("ERR not implemented".to_string())
}
