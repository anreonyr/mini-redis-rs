use crate::resp;

use super::handlers;
use super::types::{CmdError, ParsedCmd};

pub async fn dispatch_command(cmd: Result<ParsedCmd, CmdError>) -> resp::RespType {
    let parsed = match cmd {
        Ok(c) => c,
        Err(e) => return resp::RespType::Error(e.to_string()),
    };
    match parsed {
        ParsedCmd::Ping => handlers::handle_ping(),
        ParsedCmd::Echo { message } => handlers::handle_echo(&message),
        ParsedCmd::Set { key, value, expiry } => handlers::handle_set(&key, &value, expiry),
        ParsedCmd::Get { key } => handlers::handle_get(&key),
        ParsedCmd::Rpush { key, values } => handlers::handle_rpush(&key, &values),
        ParsedCmd::Lpush { key, values } => handlers::handle_lpush(&key, &values),
        ParsedCmd::Lrange { key, start, stop } => handlers::handle_lrange(&key, start, stop),
        ParsedCmd::Llen { key } => handlers::handle_llen(&key),
        ParsedCmd::Lpop { key, count } => handlers::handle_lpop(&key, count),
        ParsedCmd::Blpop { keys, timeout } => handlers::handle_blpop(&keys, timeout).await,
        ParsedCmd::Command { subcommand, name } => handlers::handle_command(subcommand, name),
        ParsedCmd::Flushdb => handlers::handle_flushdb(),
        // Streams
        ParsedCmd::Xadd { key, id, fields } => handlers::handle_xadd(&key, &id, &fields),
        ParsedCmd::Xrange { key, start, end, count } => handlers::handle_xrange(&key, &start, &end, count),
        ParsedCmd::Xrevrange { key, end, start, count } => handlers::handle_xrevrange(&key, &end, &start, count),
        ParsedCmd::Xlen { key } => handlers::handle_xlen(&key),
        ParsedCmd::Xtrim { key, strategy, threshold, exact } => handlers::handle_xtrim(&key, &strategy, threshold, exact),
        ParsedCmd::Xdel { key, ids } => handlers::handle_xdel(&key, &ids),
        ParsedCmd::Xread { count, keys, ids } => handlers::handle_xread(count, &keys, &ids),
        // Hash
        ParsedCmd::Hset { key, fields } => handlers::handle_hset(&key, &fields),
        ParsedCmd::Hget { key, field } => handlers::handle_hget(&key, &field),
        ParsedCmd::Hdel { key, fields } => handlers::handle_hdel(&key, &fields),
        ParsedCmd::Hgetall { key } => handlers::handle_hgetall(&key),
        ParsedCmd::Hexists { key, field } => handlers::handle_hexists(&key, &field),
        ParsedCmd::Hlen { key } => handlers::handle_hlen(&key),
        ParsedCmd::Hkeys { key } => handlers::handle_hkeys(&key),
        ParsedCmd::Hvals { key } => handlers::handle_hvals(&key),
        // Set
        ParsedCmd::Sadd { key, members } => handlers::handle_sadd(&key, &members),
        ParsedCmd::Smembers { key } => handlers::handle_smembers(&key),
        ParsedCmd::Sismember { key, member } => handlers::handle_sismember(&key, &member),
        ParsedCmd::Srem { key, members } => handlers::handle_srem(&key, &members),
        ParsedCmd::Scard { key } => handlers::handle_scard(&key),
        // Sorted Set
        ParsedCmd::Zadd { key, members } => handlers::handle_zadd(&key, &members),
        ParsedCmd::Zrange { key, start, stop, withscores } => {
            handlers::handle_zrange(&key, start, stop, withscores)
        }
        ParsedCmd::Zrank { key, member } => handlers::handle_zrank(&key, &member),
        ParsedCmd::Zscore { key, member } => handlers::handle_zscore(&key, &member),
    }
}
