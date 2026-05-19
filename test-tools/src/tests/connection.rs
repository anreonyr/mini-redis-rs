use crate::helpers::*;
use crate::RedisClient;
use mini_redis::protocol::resp::RespType;

pub async fn test_ping(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PING"]).await?;
    crate::assert_resp!(r, simple_str("PONG"), "PING");
    Ok(())
}

pub async fn test_echo_simple(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ECHO", "hello"]).await?;
    crate::assert_resp!(r, bulk_str("hello"), "ECHO simple");
    Ok(())
}

pub async fn test_echo_spaces(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ECHO", "hello world"]).await?;
    crate::assert_resp!(r, bulk_str("hello world"), "ECHO spaces");
    Ok(())
}

pub async fn test_unknown_command(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["FOOBAR"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("unknown") => Ok(()),
        _ => Err(format!("Unknown command: expected Error, got {}", r)),
    }
}
