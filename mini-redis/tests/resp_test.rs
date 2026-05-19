use mini_redis::protocol::resp;

#[test]
fn test_simple_string() {
    let decoder = resp::Decoder::new();
    let (val, consumed) = decoder.decode(b"+OK\r\n").unwrap();
    assert_eq!(val, resp::RespType::SimpleString("OK".to_string()));
    assert_eq!(consumed, 5);
}

#[test]
fn test_error() {
    let decoder = resp::Decoder::new();
    let (val, consumed) = decoder.decode(b"-ERR\r\n").unwrap();
    assert_eq!(val, resp::RespType::Error("ERR".to_string()));
    assert_eq!(consumed, 6);
}

#[test]
fn test_integer() {
    let decoder = resp::Decoder::new();
    let (val, consumed) = decoder.decode(b":42\r\n").unwrap();
    assert_eq!(val, resp::RespType::Integer(42));
    assert_eq!(consumed, 5);
}

#[test]
fn test_bulk_string() {
    let decoder = resp::Decoder::new();
    let data = b"$5\r\nhello\r\n";
    let (val, consumed) = decoder.decode(data).unwrap();
    assert_eq!(val, resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"hello"))));
    assert_eq!(consumed, data.len());
}

#[test]
fn test_null_bulk_string() {
    let decoder = resp::Decoder::new();
    let (val, consumed) = decoder.decode(b"$-1\r\n").unwrap();
    assert_eq!(val, resp::RespType::BulkString(None));
    assert_eq!(consumed, 5);
}

#[test]
fn test_empty_bulk_string() {
    let decoder = resp::Decoder::new();
    let data = b"$0\r\n\r\n";
    let (val, consumed) = decoder.decode(data).unwrap();
    assert_eq!(val, resp::RespType::BulkString(Some(bytes::Bytes::from_static(b""))));
    assert_eq!(consumed, data.len());
}

#[test]
fn test_array() {
    let decoder = resp::Decoder::new();
    let data = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
    let (val, consumed) = decoder.decode(data).unwrap();
    assert_eq!(
        val,
        resp::RespType::Array(Some(vec![
            resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"foo"))),
            resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"bar"))),
        ]))
    );
    assert_eq!(consumed, data.len());
}

#[test]
fn test_null_array() {
    let decoder = resp::Decoder::new();
    let (val, consumed) = decoder.decode(b"*-1\r\n").unwrap();
    assert_eq!(val, resp::RespType::Array(None));
    assert_eq!(consumed, 5);
}

#[test]
fn test_incomplete() {
    let decoder = resp::Decoder::new();
    let result = decoder.decode(b"+OK");
    assert_eq!(result, Err(resp::DecodeError::Incomplete));
}

#[test]
fn test_serialize_simple_string() {
    let val = resp::RespType::SimpleString("OK".to_string());
    assert_eq!(val.serialize(), b"+OK\r\n");
}

#[test]
fn test_serialize_bulk_string() {
    let val = resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"hello")));
    assert_eq!(val.serialize(), b"$5\r\nhello\r\n");
}

#[test]
fn test_serialize_null_bulk_string() {
    let val = resp::RespType::BulkString(None);
    assert_eq!(val.serialize(), b"$-1\r\n");
}

#[test]
fn test_serialize_array() {
    let val = resp::RespType::Array(Some(vec![
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"SET"))),
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"key"))),
        resp::RespType::BulkString(Some(bytes::Bytes::from_static(b"value"))),
    ]));
    assert_eq!(
        val.serialize(),
        b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
    );
}
