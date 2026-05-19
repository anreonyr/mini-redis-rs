use std::collections::HashMap;

use bytes::Bytes;

use crate::cmd::auth::{ConnectionState, TransactionState};
use crate::config;
use crate::db;
use crate::pubsub;
use crate::registry;
use crate::resp::RespType;
use tokio::sync::mpsc::unbounded_channel;

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
    crate::db::bump_version();
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

pub fn handle_save() -> RespType {
    let path = config::with_config(|cfg| cfg.db_path());
    match crate::persist::save(&path) {
        Ok(()) => RespType::SimpleString("OK".to_string()),
        Err(e) => RespType::Error(format!("ERR {}", e)),
    }
}

pub fn handle_bgsave() -> RespType {
    let path = config::with_config(|cfg| cfg.db_path());
    // Clone data for background saving
    let data = db::with_db(|db| {
        let now = tokio::time::Instant::now();
        let mut map: HashMap<String, db::Entry> = HashMap::new();
        for (key, entry) in db.iter() {
            if entry.expiry.is_some_and(|exp| now >= exp) {
                continue;
            }
            map.insert(key.clone(), entry.clone());
        }
        map
    });

    tokio::spawn(async move {
        let bytes = match bincode::serialize(&data) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("BGSAVE serialize error: {}", e);
                return;
            }
        };
        if let Err(e) = std::fs::write(&path, &bytes) {
            eprintln!("BGSAVE write error: {}", e);
        } else {
            println!("BGSAVE completed to {}", path);
        }
    });

    RespType::SimpleString("OK".to_string())
}

pub fn handle_shutdown() -> RespType {
    let path = config::with_config(|cfg| cfg.db_path());
    if let Err(e) = crate::persist::save(&path) {
        return RespType::Error(format!("ERR {}", e));
    }
    std::process::exit(0);
}

// Transaction handlers

pub fn handle_multi(state: &mut ConnectionState) -> RespType {
    if state.transaction.is_some() {
        return RespType::Error("ERR MULTI calls can not be nested".to_string());
    }
    state.transaction = Some(TransactionState::new());
    RespType::SimpleString("OK".to_string())
}

pub async fn handle_exec(state: &mut ConnectionState) -> RespType {
    let tx = match state.transaction.take() {
        Some(tx) => tx,
        None => return RespType::Error("ERR EXEC without MULTI".to_string()),
    };

    // Check watched keys (unwrap_or(0) matches handle_watch's unwrap_or(0) for missing keys)
    let abort = state.watching.iter().any(|(key, recorded_version)| {
        db::key_version(key).unwrap_or(0) != *recorded_version
    });
    state.watching.clear();
    if abort {
        return RespType::Array(None);
    }

    // Execute queue
    let mut results = Vec::with_capacity(tx.queue.len());
    for cmd in tx.queue {
        let response = crate::cmd::dispatch_command(Ok(cmd), state).await;
        results.push(response);
    }

    RespType::Array(Some(results))
}

pub fn handle_discard(state: &mut ConnectionState) -> RespType {
    if state.transaction.is_none() {
        return RespType::Error("ERR DISCARD without MULTI".to_string());
    }
    state.transaction = None;
    state.watching.clear();
    RespType::SimpleString("OK".to_string())
}

pub fn handle_watch(state: &mut ConnectionState, keys: &[String]) -> RespType {
    let versions: HashMap<String, u64> = keys
        .iter()
        .map(|k| (k.clone(), db::key_version(k).unwrap_or(0)))
        .collect();
    state.watching.extend(versions);
    RespType::SimpleString("OK".to_string())
}

pub fn handle_unwatch(state: &mut ConnectionState) -> RespType {
    state.watching.clear();
    RespType::SimpleString("OK".to_string())
}

// ── Pub/Sub handlers ──

pub fn handle_publish(channel: &str, message: &str) -> RespType {
    let count = pubsub::publish(channel, message);
    RespType::Integer(count as i64)
}

pub fn handle_subscribe(state: &mut ConnectionState, channels: &[String]) -> RespType {
    let (tx, rx) = unbounded_channel();
    let _count = pubsub::subscribe(tx, channels);

    state.subscription = Some(crate::cmd::auth::SubscriptionState { rx });

    // Return confirmation for the last channel subscribed
    let last = channels.last().cloned().unwrap_or_else(|| "0".to_string());
    RespType::Array(Some(vec![
        RespType::BulkString(Some(Bytes::copy_from_slice(b"subscribe"))),
        RespType::BulkString(Some(Bytes::copy_from_slice(last.as_bytes()))),
        RespType::Integer(channels.len() as i64),
    ]))
}

pub fn handle_unsubscribe(state: &mut ConnectionState, channels: &[String]) -> RespType {
    // Since we don't have the sender reference in ConnectionState yet,
    // we just clear the subscription. Real unsubscription from the
    // global registry happens when the connection drops.
    let channels_to_unsub = if channels.is_empty() {
        state.subscription = None;
        vec!["0".to_string()]
    } else {
        channels.to_vec()
    };

    let last = channels_to_unsub.last().cloned().unwrap_or_else(|| "0".to_string());
    RespType::Array(Some(vec![
        RespType::BulkString(Some(Bytes::copy_from_slice(b"unsubscribe"))),
        RespType::BulkString(Some(Bytes::copy_from_slice(last.as_bytes()))),
        RespType::Integer(0),
    ]))
}
