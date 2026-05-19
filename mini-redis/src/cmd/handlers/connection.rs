use bytes::Bytes;

use crate::registry;
use crate::resp::RespType;

pub fn handle_ping() -> RespType {
    RespType::SimpleString("PONG".to_string())
}

pub fn handle_echo(message: &str) -> RespType {
    RespType::BulkString(Some(Bytes::copy_from_slice(message.as_bytes())))
}

pub fn handle_command(subcommand: Option<String>, name: Option<String>) -> RespType {
    match subcommand.as_deref() {
        Some("INFO") => {
            if let Some(n) = name {
                let info = registry::with_registry(|reg| {
                    reg.get(&n).map(|ci| {
                        let mut arr = Vec::new();
                        arr.push(RespType::BulkString(Some(Bytes::copy_from_slice(
                            ci.name.as_bytes(),
                        ))));
                        arr.push(RespType::Integer(ci.arity as i64));
                        arr.push(RespType::Array(Some(vec![])));
                        arr.push(RespType::Integer(0));
                        arr.push(RespType::Integer(if ci.arity.abs() > 1 {
                            ci.arity.unsigned_abs() as i64 - 1
                        } else {
                            0
                        }));
                        arr.push(RespType::Integer(1));
                        RespType::Array(Some(arr))
                    })
                });
                match info {
                    Some(item) => RespType::Array(Some(vec![item])),
                    None => RespType::Array(None),
                }
            } else {
                let infos = registry::with_registry(|reg| {
                    reg.list_all()
                        .iter()
                        .map(|ci| {
                            let mut arr = Vec::new();
                            arr.push(RespType::BulkString(Some(Bytes::copy_from_slice(
                                ci.name.as_bytes(),
                            ))));
                            arr.push(RespType::Integer(ci.arity as i64));
                            arr.push(RespType::Array(Some(vec![])));
                            arr.push(RespType::Integer(0));
                            arr.push(RespType::Integer(if ci.arity.abs() > 1 {
                                ci.arity.unsigned_abs() as i64 - 1
                            } else {
                                0
                            }));
                            arr.push(RespType::Integer(1));
                            RespType::Array(Some(arr))
                        })
                        .collect::<Vec<_>>()
                });
                RespType::Array(Some(infos))
            }
        }
        _ => {
            let names = registry::with_registry(|reg| {
                reg.list_all()
                    .iter()
                    .map(|ci| {
                        RespType::BulkString(Some(Bytes::copy_from_slice(ci.name.as_bytes())))
                    })
                    .collect::<Vec<_>>()
            });
            RespType::Array(Some(names))
        }
    }
}

pub fn handle_flushdb() -> RespType {
    crate::db::flushdb();
    RespType::SimpleString("OK".to_string())
}

pub fn handle_info(section: Option<String>) -> RespType {
    let text = match section.as_deref() {
        Some("server") => "# Server\r\nredis_version:0.1.0\r\n",
        _ => "# Server\r\nredis_version:0.1.0\r\n",
    };
    RespType::BulkString(Some(bytes::Bytes::copy_from_slice(text.as_bytes())))
}

pub fn handle_config_get(parameter: &str) -> RespType {
    let value = match parameter.to_lowercase().as_str() {
        "dir" => ".",
        "dbfilename" => "dump.rdb",
        "maxclients" => "10000",
        "databases" => "1",
        _ => return RespType::Array(Some(vec![])),
    };
    RespType::Array(Some(vec![
        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(parameter.as_bytes()))),
        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(value.as_bytes()))),
    ]))
}
