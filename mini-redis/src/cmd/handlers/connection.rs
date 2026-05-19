use bytes::Bytes;

use crate::config;
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
    match parameter.to_lowercase().as_str() {
        "dir" => {
            let val = config::with_config(|cfg| cfg.dir.clone());
            RespType::Array(Some(vec![
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(parameter.as_bytes()))),
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(val.as_bytes()))),
            ]))
        }
        "dbfilename" => {
            let val = config::with_config(|cfg| cfg.dbfilename.clone());
            RespType::Array(Some(vec![
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(parameter.as_bytes()))),
                RespType::BulkString(Some(bytes::Bytes::copy_from_slice(val.as_bytes()))),
            ]))
        }
        "maxclients" => RespType::Array(Some(vec![
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(parameter.as_bytes()))),
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"10000"))),
        ])),
        "databases" => RespType::Array(Some(vec![
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(parameter.as_bytes()))),
            RespType::BulkString(Some(bytes::Bytes::copy_from_slice(b"1"))),
        ])),
        "requirepass" => {
            let pw = config::with_config(|cfg| cfg.requirepass.clone());
            match pw {
                Some(p) => RespType::Array(Some(vec![
                    RespType::BulkString(Some(bytes::Bytes::copy_from_slice(
                        parameter.as_bytes(),
                    ))),
                    RespType::BulkString(Some(bytes::Bytes::copy_from_slice(p.as_bytes()))),
                ])),
                None => RespType::Array(Some(vec![])),
            }
        }
        _ => RespType::Array(Some(vec![])),
    }
}

pub fn handle_config_set(parameter: &str, value: &str) -> RespType {
    match parameter.to_lowercase().as_str() {
        "requirepass" => {
            let pw = if value.is_empty() || value == "\"\"" {
                None
            } else {
                Some(value.to_string())
            };
            config::with_config_mut(|cfg| cfg.requirepass = pw);
            RespType::SimpleString("OK".to_string())
        }
        "dir" => {
            config::with_config_mut(|cfg| cfg.dir = value.to_string());
            RespType::SimpleString("OK".to_string())
        }
        "dbfilename" => {
            config::with_config_mut(|cfg| cfg.dbfilename = value.to_string());
            RespType::SimpleString("OK".to_string())
        }
        _ => RespType::Error("ERR unknown config parameter".to_string()),
    }
}
