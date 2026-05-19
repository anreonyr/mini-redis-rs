use crate::config;
use crate::resp;

use super::auth::{self, ConnectionState};
use super::handlers;
use super::types::{CmdError, ParsedCmd};

pub async fn dispatch_command(
    cmd: Result<ParsedCmd, CmdError>,
    state: &mut ConnectionState,
) -> resp::RespType {
    let parsed = match cmd {
        Ok(c) => c,
        Err(e) => return resp::RespType::Error(e.to_string()),
    };

    // Auth check: if requirepass is set and not authenticated and not a bypass command, reject
    if !state.is_authenticated()
        && config::with_config(|cfg| cfg.requirepass_is_set())
        && !auth::is_allowed_before_auth(parsed.name())
    {
        return resp::RespType::Error("NOAUTH Authentication required.".to_string());
    }

    match parsed {
        ParsedCmd::Ping => handlers::handle_ping(),
        ParsedCmd::Echo { message } => handlers::handle_echo(&message),
        ParsedCmd::Set { key, value, expiry } => handlers::handle_set(&key, &value, expiry),
        ParsedCmd::Get { key } => handlers::handle_get(&key),
        ParsedCmd::Incr { key } => handlers::handle_incr(&key),
        ParsedCmd::Decr { key } => handlers::handle_decr(&key),
        ParsedCmd::Incrby { key, delta } => handlers::handle_incrby(&key, delta),
        ParsedCmd::Decrby { key, delta } => handlers::handle_decrby(&key, delta),
        ParsedCmd::Append { key, value } => handlers::handle_append(&key, &value),
        ParsedCmd::Strlen { key } => handlers::handle_strlen(&key),
        ParsedCmd::Mget { keys } => handlers::handle_mget(&keys),
        ParsedCmd::Mset { pairs } => handlers::handle_mset(&pairs),
        ParsedCmd::Getset { key, value } => handlers::handle_getset(&key, &value),
        ParsedCmd::Getrange { key, start, end } => handlers::handle_getrange(&key, start, end),
        ParsedCmd::Setrange { key, offset, value } => {
            handlers::handle_setrange(&key, offset, &value)
        }
        ParsedCmd::Msetnx { pairs } => handlers::handle_msetnx(&pairs),
        ParsedCmd::Rpush { key, values } => handlers::handle_rpush(&key, &values),
        ParsedCmd::Lpush { key, values } => handlers::handle_lpush(&key, &values),
        ParsedCmd::Lrange { key, start, stop } => handlers::handle_lrange(&key, start, stop),
        ParsedCmd::Llen { key } => handlers::handle_llen(&key),
        ParsedCmd::Lpop { key, count } => handlers::handle_lpop(&key, count),
        ParsedCmd::Rpop { key, count } => handlers::handle_rpop(&key, count),
        ParsedCmd::Lindex { key, index } => handlers::handle_lindex(&key, index),
        ParsedCmd::Lrem { key, count, value } => handlers::handle_lrem(&key, count, &value),
        ParsedCmd::Ltrim { key, start, stop } => handlers::handle_ltrim(&key, start, stop),
        ParsedCmd::Rpoplpush { source, destination } => {
            handlers::handle_rpoplpush(&source, &destination)
        }
        ParsedCmd::Lset { key, index, value } => handlers::handle_lset(&key, index, &value),
        ParsedCmd::Blpop { keys, timeout } => handlers::handle_blpop(&keys, timeout).await,
        ParsedCmd::Command { subcommand, name } => handlers::handle_command(subcommand, name),
        ParsedCmd::Flushdb => handlers::handle_flushdb(),
        ParsedCmd::Info { section } => handlers::handle_info(section),
        ParsedCmd::ConfigGet { parameter } => handlers::handle_config_get(&parameter),
        ParsedCmd::ConfigSet { parameter, value } => {
            handlers::handle_config_set(&parameter, &value)
        }
        // Auth
        ParsedCmd::Auth { password } => auth::handle_auth(state, &password),
        // Streams
        ParsedCmd::Xadd { key, id, fields } => handlers::handle_xadd(&key, &id, &fields),
        ParsedCmd::Xrange {
            key,
            start,
            end,
            count,
        } => handlers::handle_xrange(&key, &start, &end, count),
        ParsedCmd::Xrevrange {
            key,
            end,
            start,
            count,
        } => handlers::handle_xrevrange(&key, &end, &start, count),
        ParsedCmd::Xlen { key } => handlers::handle_xlen(&key),
        ParsedCmd::Xtrim {
            key,
            strategy,
            threshold,
            exact,
        } => handlers::handle_xtrim(&key, &strategy, threshold, exact),
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
        ParsedCmd::Hincrby { key, field, incr } => handlers::handle_hincrby(&key, &field, incr),
        ParsedCmd::Hincrbyfloat { key, field, incr } => {
            handlers::handle_hincrbyfloat(&key, &field, incr)
        }
        ParsedCmd::Hsetnx { key, field, value } => handlers::handle_hsetnx(&key, &field, &value),
        // Set
        ParsedCmd::Sadd { key, members } => handlers::handle_sadd(&key, &members),
        ParsedCmd::Smembers { key } => handlers::handle_smembers(&key),
        ParsedCmd::Sismember { key, member } => handlers::handle_sismember(&key, &member),
        ParsedCmd::Srem { key, members } => handlers::handle_srem(&key, &members),
        ParsedCmd::Scard { key } => handlers::handle_scard(&key),
        ParsedCmd::Spop { key, count } => handlers::handle_spop(&key, count),
        ParsedCmd::Srandmember { key, count } => handlers::handle_srandmember(&key, count),
        ParsedCmd::Sunion { keys } => handlers::handle_sunion(&keys),
        ParsedCmd::Sinter { keys } => handlers::handle_sinter(&keys),
        ParsedCmd::Sdiff { keys } => handlers::handle_sdiff(&keys),
        ParsedCmd::Smove {
            source,
            destination,
            member,
        } => handlers::handle_smove(&source, &destination, &member),
        // Sorted Set
        ParsedCmd::Zadd { key, members } => handlers::handle_zadd(&key, &members),
        ParsedCmd::Zrange {
            key,
            start,
            stop,
            withscores,
        } => handlers::handle_zrange(&key, start, stop, withscores),
        ParsedCmd::Zrank { key, member } => handlers::handle_zrank(&key, &member),
        ParsedCmd::Zscore { key, member } => handlers::handle_zscore(&key, &member),
        ParsedCmd::Zrem { key, members } => handlers::handle_zrem(&key, &members),
        ParsedCmd::Zcard { key } => handlers::handle_zcard(&key),
        ParsedCmd::Zcount { key, min, max } => handlers::handle_zcount(&key, &min, &max),
        ParsedCmd::Zrangebyscore {
            key,
            min,
            max,
            withscores,
            limit,
        } => handlers::handle_zrangebyscore(&key, &min, &max, withscores, limit),
        ParsedCmd::Zincrby { key, incr, member } => {
            handlers::handle_zincrby(&key, incr, &member)
        }
        ParsedCmd::Zrevrange {
            key,
            start,
            stop,
            withscores,
        } => handlers::handle_zrevrange(&key, start, stop, withscores),
        ParsedCmd::Zrevrank { key, member } => handlers::handle_zrevrank(&key, &member),
        ParsedCmd::Zremrangebyrank { key, start, stop } => {
            handlers::handle_zremrangebyrank(&key, start, stop)
        }
        ParsedCmd::Zremrangebyscore { key, min, max } => {
            handlers::handle_zremrangebyscore(&key, &min, &max)
        }
        ParsedCmd::Zrevrangebyscore {
            key,
            max,
            min,
            withscores,
            limit,
        } => handlers::handle_zrevrangebyscore(&key, &max, &min, withscores, limit),
        // Key Management
        ParsedCmd::Del { keys } => handlers::handle_del(&keys),
        ParsedCmd::Exists { keys } => handlers::handle_exists(&keys),
        ParsedCmd::Type { key } => handlers::handle_type(&key),
        ParsedCmd::Keys { pattern } => handlers::handle_keys(&pattern),
        ParsedCmd::Dbsize => handlers::handle_dbsize(),
        // Expiry Management
        ParsedCmd::Expire { key, seconds } => handlers::handle_expire(&key, seconds),
        ParsedCmd::Ttl { key } => handlers::handle_ttl(&key),
        ParsedCmd::Persist { key } => handlers::handle_persist(&key),
        // More Key
        ParsedCmd::Rename { key, newkey } => handlers::handle_rename(&key, &newkey),
        ParsedCmd::Renamenx { key, newkey } => handlers::handle_renamenx(&key, &newkey),
        ParsedCmd::Randomkey => handlers::handle_randomkey(),
        ParsedCmd::Save => handlers::handle_save(),
        ParsedCmd::Bgsave => handlers::handle_bgsave(),
        ParsedCmd::Shutdown => handlers::handle_shutdown(),
    }
}
