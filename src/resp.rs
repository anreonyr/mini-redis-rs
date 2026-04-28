use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum RespType {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Option<Vec<u8>>),
    Array(Option<Vec<RespType>>),
}

impl RespType {
    pub fn serialize(&self) -> Vec<u8> {
        match self {
            RespType::SimpleString(s) => format!("+{}\r\n", s).into_bytes(),
            RespType::Error(e) => format!("-{}\r\n", e).into_bytes(),
            RespType::Integer(i) => format!(":{}\r\n", i).into_bytes(),
            RespType::BulkString(Some(data)) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(format!("${}\r\n", data.len()).as_bytes());
                buf.extend_from_slice(data);
                buf.extend_from_slice(b"\r\n");
                buf
            }
            RespType::BulkString(None) => b"$-1\r\n".to_vec(),
            RespType::Array(Some(items)) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(format!("*{}\r\n", items.len()).as_bytes());
                for item in items {
                    buf.extend_from_slice(&item.serialize());
                }
                buf
            }
            RespType::Array(None) => b"*-1\r\n".to_vec(),
        }
    }
}

impl fmt::Display for RespType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RespType::SimpleString(s) => write!(f, "SimpleString({})", s),
            RespType::Error(e) => write!(f, "Error({})", e),
            RespType::Integer(i) => write!(f, "Integer({})", i),
            RespType::BulkString(Some(data)) => {
                let s = String::from_utf8_lossy(data);
                write!(f, "BulkString({})", s)
            }
            RespType::BulkString(None) => write!(f, "BulkString(null)"),
            RespType::Array(Some(items)) => {
                write!(f, "Array([")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "])")
            }
            RespType::Array(None) => write!(f, "Array(null)"),
        }
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum DecodeError {
    #[error("incomplete frame: need more data")]
    Incomplete,
    #[error("invalid frame: {0}")]
    Invalid(String),
}

pub struct Decoder;

impl Decoder {
    pub fn new() -> Self {
        Decoder
    }

    pub fn decode(&self, buf: &[u8]) -> Result<(RespType, usize), DecodeError> {
        if buf.is_empty() {
            return Err(DecodeError::Incomplete);
        }
        match buf[0] {
            b'+' => self.decode_simple_string(buf),
            b'-' => self.decode_error(buf),
            b':' => self.decode_integer(buf),
            b'$' => self.decode_bulk_string(buf),
            b'*' => self.decode_array(buf),
            b => Err(DecodeError::Invalid(format!(
                "Unknown RESP type byte: 0x{:02x} ('{}')",
                b, b as char
            ))),
        }
    }

    fn decode_simple_string(&self, buf: &[u8]) -> Result<(RespType, usize), DecodeError> {
        let (line, consumed) = self.read_line(&buf[1..])?;
        Ok((RespType::SimpleString(line), consumed + 1))
    }

    fn decode_error(&self, buf: &[u8]) -> Result<(RespType, usize), DecodeError> {
        let (line, consumed) = self.read_line(&buf[1..])?;
        Ok((RespType::Error(line), consumed + 1))
    }

    fn decode_integer(&self, buf: &[u8]) -> Result<(RespType, usize), DecodeError> {
        let (line, consumed) = self.read_line(&buf[1..])?;
        let val: i64 = line
            .parse()
            .map_err(|_| DecodeError::Invalid(format!("Invalid integer: {}", line)))?;
        Ok((RespType::Integer(val), consumed + 1))
    }

    fn decode_bulk_string(&self, buf: &[u8]) -> Result<(RespType, usize), DecodeError> {
        let (len_str, consumed) = self.read_line(&buf[1..])?;
        let len: i64 = len_str.parse().map_err(|_| {
            DecodeError::Invalid(format!("Invalid bulk string length: {}", len_str))
        })?;

        // Null bulk string
        if len == -1 {
            return Ok((RespType::BulkString(None), consumed + 1));
        }
        if len < 0 {
            return Err(DecodeError::Invalid(format!(
                "Negative bulk string length: {}",
                len
            )));
        }

        let len = len as usize;
        let total = 1 + consumed + len + 2; // $ + \r\n + data + \r\n
        if buf.len() < total {
            return Err(DecodeError::Incomplete);
        }

        let data = buf[(1 + consumed)..(1 + consumed + len)].to_vec();
        // Check trailing \r\n
        if buf[1 + consumed + len..1 + consumed + len + 2] != *b"\r\n" {
            return Err(DecodeError::Invalid(
                "Missing CRLF after bulk string data".to_string(),
            ));
        }

        Ok((RespType::BulkString(Some(data)), total))
    }

    fn decode_array(&self, buf: &[u8]) -> Result<(RespType, usize), DecodeError> {
        let (count_str, consumed) = self.read_line(&buf[1..])?;
        let count: i64 = count_str
            .parse()
            .map_err(|_| DecodeError::Invalid(format!("Invalid array length: {}", count_str)))?;

        // Null array
        if count == -1 {
            return Ok((RespType::Array(None), consumed + 1));
        }
        if count < 0 {
            return Err(DecodeError::Invalid(format!(
                "Negative array length: {}",
                count
            )));
        }

        let count = count as usize;
        let mut total = 1 + consumed;
        let mut items = Vec::with_capacity(count);

        for _ in 0..count {
            if total >= buf.len() {
                return Err(DecodeError::Incomplete);
            }
            let (item, item_consumed) = self.decode(&buf[total..])?;
            items.push(item);
            total += item_consumed;
        }

        Ok((RespType::Array(Some(items)), total))
    }

    fn read_line(&self, buf: &[u8]) -> Result<(String, usize), DecodeError> {
        if let Some(pos) = buf.windows(2).position(|w| w == b"\r\n") {
            let line = std::str::from_utf8(&buf[..pos])
                .map_err(|e| DecodeError::Invalid(format!("Invalid UTF-8: {}", e)))?
                .to_string();
            Ok((line, pos + 2))
        } else {
            Err(DecodeError::Incomplete)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let decoder = Decoder::new();
        let (val, consumed) = decoder.decode(b"+OK\r\n").unwrap();
        assert_eq!(val, RespType::SimpleString("OK".to_string()));
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_error() {
        let decoder = Decoder::new();
        let (val, consumed) = decoder.decode(b"-ERR\r\n").unwrap();
        assert_eq!(val, RespType::Error("ERR".to_string()));
        assert_eq!(consumed, 6);
    }

    #[test]
    fn test_integer() {
        let decoder = Decoder::new();
        let (val, consumed) = decoder.decode(b":42\r\n").unwrap();
        assert_eq!(val, RespType::Integer(42));
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_bulk_string() {
        let decoder = Decoder::new();
        let data = b"$5\r\nhello\r\n";
        let (val, consumed) = decoder.decode(data).unwrap();
        assert_eq!(val, RespType::BulkString(Some(b"hello".to_vec())));
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn test_null_bulk_string() {
        let decoder = Decoder::new();
        let (val, consumed) = decoder.decode(b"$-1\r\n").unwrap();
        assert_eq!(val, RespType::BulkString(None));
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_empty_bulk_string() {
        let decoder = Decoder::new();
        let data = b"$0\r\n\r\n";
        let (val, consumed) = decoder.decode(data).unwrap();
        assert_eq!(val, RespType::BulkString(Some(b"".to_vec())));
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn test_array() {
        let decoder = Decoder::new();
        let data = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let (val, consumed) = decoder.decode(data).unwrap();
        assert_eq!(
            val,
            RespType::Array(Some(vec![
                RespType::BulkString(Some(b"foo".to_vec())),
                RespType::BulkString(Some(b"bar".to_vec())),
            ]))
        );
        assert_eq!(consumed, data.len());
    }

    #[test]
    fn test_null_array() {
        let decoder = Decoder::new();
        let (val, consumed) = decoder.decode(b"*-1\r\n").unwrap();
        assert_eq!(val, RespType::Array(None));
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_incomplete() {
        let decoder = Decoder::new();
        let result = decoder.decode(b"+OK");
        assert_eq!(result, Err(DecodeError::Incomplete));
    }

    #[test]
    fn test_serialize_simple_string() {
        let val = RespType::SimpleString("OK".to_string());
        assert_eq!(val.serialize(), b"+OK\r\n");
    }

    #[test]
    fn test_serialize_bulk_string() {
        let val = RespType::BulkString(Some(b"hello".to_vec()));
        assert_eq!(val.serialize(), b"$5\r\nhello\r\n");
    }

    #[test]
    fn test_serialize_null_bulk_string() {
        let val = RespType::BulkString(None);
        assert_eq!(val.serialize(), b"$-1\r\n");
    }

    #[test]
    fn test_serialize_array() {
        let val = RespType::Array(Some(vec![
            RespType::BulkString(Some(b"SET".to_vec())),
            RespType::BulkString(Some(b"key".to_vec())),
            RespType::BulkString(Some(b"value".to_vec())),
        ]));
        assert_eq!(
            val.serialize(),
            b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n"
        );
    }
}
