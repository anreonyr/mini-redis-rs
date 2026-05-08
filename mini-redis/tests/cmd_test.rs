use mini_redis::cmd::ParsedCmd;
use mini_redis::{cmd, resp};

#[tokio::test]
async fn test_dispatch_command_ping() {
    assert_eq!(
        cmd::dispatch_command(ParsedCmd::parse("PING", vec![])).await,
        resp::RespType::SimpleString("PONG".to_string())
    );
}

#[tokio::test]
async fn test_dispatch_command_echo() {
    assert_eq!(
        cmd::dispatch_command(ParsedCmd::parse("ECHO", vec!["foo".to_string()])).await,
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"foo")))
    );
}

#[tokio::test]
async fn test_dispatch_command_unknown() {
    assert_eq!(
        cmd::dispatch_command(ParsedCmd::parse("UNKNOWN", vec![])).await,
        resp::RespType::Error("ERR unknown command".to_string())
    );
}

#[test]
fn test_parse_command() {
    let frame = resp::RespType::Array(Some(vec![
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"GET"))),
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"key"))),
    ]));
    let result = cmd::parse_command(&frame).unwrap();
    assert_eq!(result.unwrap(), ParsedCmd::Get { key: "key".to_string() });
}

#[test]
fn test_parse_command_not_array() {
    let frame = resp::RespType::SimpleString("OK".to_string());
    assert!(cmd::parse_command(&frame).is_none());
}

#[tokio::test]
async fn test_set_get_roundtrip() {
    let result = cmd::dispatch_command(ParsedCmd::parse("SET", vec!["rt1".to_string(), "val1".to_string()])).await;
    assert_eq!(result, resp::RespType::SimpleString("OK".to_string()));

    let result = cmd::dispatch_command(ParsedCmd::parse("GET", vec!["rt1".to_string()])).await;
    assert_eq!(result, resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"val1"))));
}

#[tokio::test]
async fn test_get_nonexistent_key() {
    let result = cmd::dispatch_command(ParsedCmd::parse("GET", vec!["nonexistent".to_string()])).await;
    assert_eq!(result, resp::RespType::BulkString(None));
}

#[tokio::test]
async fn test_get_no_args() {
    let result = cmd::dispatch_command(ParsedCmd::parse("GET", vec![])).await;
    assert_eq!(
        result,
        resp::RespType::Error("ERR wrong number of arguments for 'get' command".to_string())
    );
}

#[tokio::test]
async fn test_echo_no_args() {
    let result = cmd::dispatch_command(ParsedCmd::parse("ECHO", vec![])).await;
    assert_eq!(
        result,
        resp::RespType::Error("ERR wrong number of arguments for 'echo' command".to_string())
    );
}

#[tokio::test]
async fn test_set_with_invalid_expiry() {
    let result = cmd::dispatch_command(ParsedCmd::parse(
        "SET",
        vec![
            "k".to_string(),
            "v".to_string(),
            "EX".to_string(),
            "not_a_number".to_string(),
        ],
    ))
    .await;
    assert_eq!(
        result,
        resp::RespType::Error("ERR value is not an integer or out of range".to_string())
    );
}

#[tokio::test]
async fn test_set_with_unknown_flag() {
    let result = cmd::dispatch_command(ParsedCmd::parse(
        "SET",
        vec![
            "k".to_string(),
            "v".to_string(),
            "XX".to_string(),
            "100".to_string(),
        ],
    ))
    .await;
    assert_eq!(
        result,
        resp::RespType::Error("ERR syntax error".to_string())
    );
}
