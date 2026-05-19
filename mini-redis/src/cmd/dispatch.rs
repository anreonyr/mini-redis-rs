use std::future::Future;
use std::pin::Pin;

use crate::config;
use crate::resp;

use super::auth::{self, ConnectionState};
use super::handlers;
use super::types::{CmdError, ParsedCmd};

/// Dispatch a parsed command to its handler.
///
/// Returns a boxed future to break potential async recursion through EXEC
/// (dispatch_command -> handle_exec -> dispatch_command -> ...).
pub fn dispatch_command<'a>(
    cmd: Result<ParsedCmd, CmdError>,
    state: &'a mut ConnectionState,
) -> Pin<Box<dyn Future<Output = resp::RespType> + Send + 'a>> {
    let parsed = match cmd {
        Ok(c) => c,
        Err(e) => return Box::pin(async move { resp::RespType::Error(e.to_string()) }),
    };

    // Set current database for this connection
    crate::db::set_current_db(state.db_index);

    // Auth check: if requirepass is set and not authenticated and not a bypass command, reject
    if !state.is_authenticated()
        && config::with_config(|cfg| cfg.requirepass_is_set())
        && !auth::is_allowed_before_auth(parsed.name())
    {
        return Box::pin(async {
            resp::RespType::Error("NOAUTH Authentication required.".to_string())
        });
    }

    // Transaction queueing: if in a transaction and command is queueable
    if let Some(ref mut tx) = state.transaction {
        let bypass = matches!(&parsed,
            ParsedCmd::Multi | ParsedCmd::Exec
            | ParsedCmd::Discard | ParsedCmd::Watch { .. }
            | ParsedCmd::Unwatch
        );
        if !bypass {
            tx.queue.push(parsed);
            return Box::pin(async { resp::RespType::SimpleString("QUEUED".to_string()) });
        }
    }

    Box::pin(dispatch_match(parsed, state))
}

/// The inner async dispatch match. Separated from `dispatch_command` so that
/// the latter can return a boxed future, breaking async recursion through EXEC.
async fn dispatch_match<'a>(
    parsed: ParsedCmd,
    state: &'a mut ConnectionState,
) -> resp::RespType {
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
        // Consumer Groups
        ParsedCmd::XGroup { sub, key } => {
            handlers::handle_xgroup(sub, &key)
        }
        ParsedCmd::XReadGroup { group, consumer, count, keys, ids } => {
            handlers::handle_xreadgroup(&group, &consumer, count, &keys, &ids)
        }
        ParsedCmd::XAck { key, group, ids } => {
            handlers::handle_xack(&key, &group, &ids)
        }
        ParsedCmd::XPending { key, group, start, end, count, consumer } => {
            handlers::handle_xpending(&key, &group, &start, &end, count, consumer.as_deref())
        }
        ParsedCmd::XClaim { key, group, consumer, min_idle, ids } => {
            handlers::handle_xclaim(&key, &group, &consumer, min_idle, &ids)
        }
        ParsedCmd::XInfo { sub, key, group } => {
            handlers::handle_xinfo(&sub, &key, group.as_deref())
        }
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
        // ZSet Set Operations
        ParsedCmd::ZInter {
            numkeys,
            keys,
            weights,
            aggregate,
            withscores,
        } => handlers::handle_zinter(numkeys, &keys, &weights, &aggregate, withscores),
        ParsedCmd::ZInterStore {
            dest,
            numkeys,
            keys,
            weights,
            aggregate,
        } => handlers::handle_zinterstore(&dest, numkeys, &keys, &weights, &aggregate),
        ParsedCmd::ZUnion {
            numkeys,
            keys,
            weights,
            aggregate,
            withscores,
        } => handlers::handle_zunion(numkeys, &keys, &weights, &aggregate, withscores),
        ParsedCmd::ZUnionStore {
            dest,
            numkeys,
            keys,
            weights,
            aggregate,
        } => handlers::handle_zunionstore(&dest, numkeys, &keys, &weights, &aggregate),
        ParsedCmd::ZDiff { keys, withscores, .. } => {
            handlers::handle_zdiff(&keys, withscores)
        }
        ParsedCmd::ZDiffStore { dest, keys } => {
            handlers::handle_zdiffstore(&dest, &keys)
        }
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
        ParsedCmd::Save => handlers::handle_save().await,
        ParsedCmd::Bgsave => handlers::handle_bgsave(),
        ParsedCmd::Shutdown => handlers::handle_shutdown().await,
        // Geo
        ParsedCmd::GeoAdd { key, members } => handlers::handle_geoadd(&key, &members),
        ParsedCmd::GeoDist { key, member1, member2, unit } => handlers::handle_geodist(&key, &member1, &member2, &unit),
        ParsedCmd::GeoHash { key, members } => handlers::handle_geohash(&key, &members),
        ParsedCmd::GeoPos { key, members } => handlers::handle_geopos(&key, &members),
        ParsedCmd::GeoRadius { key, longitude, latitude, radius, unit, withcoord, withdist, count } => {
            handlers::handle_georadius(&key, longitude, latitude, radius, &unit, withcoord, withdist, count)
        }
        ParsedCmd::GeoRadiusByMember { key, member, radius, unit, withcoord, withdist, count } => {
            handlers::handle_georadiusbymember(&key, &member, radius, &unit, withcoord, withdist, count)
        }
        // Transaction
        ParsedCmd::Multi => handlers::handle_multi(state),
        ParsedCmd::Exec => handlers::handle_exec(state).await,
        ParsedCmd::Discard => handlers::handle_discard(state),
        ParsedCmd::Watch { keys } => handlers::handle_watch(state, &keys),
        ParsedCmd::Unwatch => handlers::handle_unwatch(state),
        // Pub/Sub
        ParsedCmd::Publish { channel, message } => {
            handlers::handle_publish(&channel, &message)
        }
        ParsedCmd::Subscribe { channels } => {
            handlers::handle_subscribe(state, &channels)
        }
        ParsedCmd::Unsubscribe { channels } => {
            handlers::handle_unsubscribe(state, &channels)
        }
        // Connection management
        ParsedCmd::Select { index } => handlers::handle_select(state, index),
        ParsedCmd::Quit => handlers::handle_quit(state),
        ParsedCmd::ClientSetName { name } => handlers::handle_client_setname(state, &name),
        ParsedCmd::ClientGetName => handlers::handle_client_getname(state),
        ParsedCmd::Hello => handlers::handle_hello(),
        // Bitmap
        ParsedCmd::GetBit { key, offset } => handlers::handle_getbit(&key, offset),
        ParsedCmd::SetBit { key, offset, value } => handlers::handle_setbit(&key, offset, value),
        ParsedCmd::BitCount { key, start, end } => handlers::handle_bitcount(&key, start, end),
        ParsedCmd::BitOp { op, dest, keys } => handlers::handle_bitop(&op, &dest, &keys),
        ParsedCmd::BitPos { key, bit, start, end } => handlers::handle_bitpos(&key, bit, start, end),
        // Scan
        ParsedCmd::Scan { cursor, match_pattern, count, type_filter } => {
            handlers::handle_scan(cursor, match_pattern, count, type_filter)
        }
        ParsedCmd::Sscan { key, cursor, match_pattern, count } => {
            handlers::handle_sscan(&key, cursor, match_pattern, count)
        }
        ParsedCmd::Hscan { key, cursor, match_pattern, count } => {
            handlers::handle_hscan(&key, cursor, match_pattern, count)
        }
        ParsedCmd::Zscan { key, cursor, match_pattern, count } => {
            handlers::handle_zscan(&key, cursor, match_pattern, count)
        }
    }
}
