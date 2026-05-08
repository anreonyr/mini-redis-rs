use codecrafters_redis::cmd;
use codecrafters_redis::resp;

#[test]
fn test_dispatch_command_ping() {
    assert_eq!(
        cmd::dispatch_command("PING", &[]),
        resp::RespType::SimpleString("PONG".to_string())
    );
}

#[test]
fn test_dispatch_command_echo() {
    assert_eq!(
        cmd::dispatch_command("ECHO", &["foo".to_string()]),
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"foo")))
    );
}

#[test]
fn test_dispatch_command_unknown() {
    assert_eq!(
        cmd::dispatch_command("UNKNOWN", &[]),
        resp::RespType::Error("ERR unknown command".to_string())
    );
}

#[test]
fn test_parse_command() {
    let frame = resp::RespType::Array(Some(vec![
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"GET"))),
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"key"))),
    ]));
    let (parsed_cmd, args) = cmd::parse_command(&frame).unwrap();
    assert_eq!(parsed_cmd, "GET");
    assert_eq!(args, vec!["key".to_string()]);
}

#[test]
fn test_parse_command_not_array() {
    let frame = resp::RespType::SimpleString("OK".to_string());
    assert!(cmd::parse_command(&frame).is_none());
}

#[test]
fn test_set_get_roundtrip() {
    let result = cmd::dispatch_command("SET", &["rt1".to_string(), "val1".to_string()]);
    assert_eq!(result, resp::RespType::SimpleString("OK".to_string()));

    let result = cmd::dispatch_command("GET", &["rt1".to_string()]);
    assert_eq!(result, resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"val1"))));
}

#[test]
fn test_get_nonexistent_key() {
    let result = cmd::dispatch_command("GET", &["nonexistent".to_string()]);
    assert_eq!(result, resp::RespType::BulkString(None));
}

#[test]
fn test_get_no_args() {
    let result = cmd::dispatch_command("GET", &[]);
    assert_eq!(
        result,
        resp::RespType::Error("ERR wrong number of arguments for 'get' command".to_string())
    );
}

#[test]
fn test_echo_no_args() {
    let result = cmd::dispatch_command("ECHO", &[]);
    assert_eq!(
        result,
        resp::RespType::Error("ERR wrong number of arguments for 'echo' command".to_string())
    );
}

#[test]
fn test_set_with_invalid_expiry() {
    let result = cmd::dispatch_command(
        "SET",
        &[
            "k".to_string(),
            "v".to_string(),
            "EX".to_string(),
            "not_a_number".to_string(),
        ],
    );
    assert_eq!(
        result,
        resp::RespType::Error("ERR value is not an integer or out of range".to_string())
    );
}

#[test]
fn test_set_with_unknown_flag() {
    let result = cmd::dispatch_command(
        "SET",
        &[
            "k".to_string(),
            "v".to_string(),
            "XX".to_string(),
            "100".to_string(),
        ],
    );
    assert_eq!(
        result,
        resp::RespType::Error("ERR syntax error".to_string())
    );
}
