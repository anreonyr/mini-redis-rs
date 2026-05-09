use std::time::Duration;

/// All arguments have been parsed and validated at this point.
#[derive(Debug, PartialEq)]
pub enum ParsedCmd {
    Ping,
    Echo {
        message: String,
    },
    Set {
        key: String,
        value: String,
        expiry: Option<Duration>,
    },
    Get {
        key: String,
    },
    Rpush {
        key: String,
        values: Vec<String>,
    },
    Lpush {
        key: String,
        values: Vec<String>,
    },
    Lrange {
        key: String,
        start: i64,
        stop: i64,
    },
    Llen {
        key: String,
    },
    Lpop {
        key: String,
        count: Option<usize>,
    },
    Flushdb,
    Blpop {
        keys: Vec<String>,
        timeout: u64,
    },
    Command {
        subcommand: Option<String>,
        name: Option<String>,
    },
    // Streams
    Xadd {
        key: String,
        id: String,
        fields: Vec<String>,
    },
    Xrange {
        key: String,
        start: String,
        end: String,
        count: Option<u64>,
    },
    Xrevrange {
        key: String,
        end: String,
        start: String,
        count: Option<u64>,
    },
    Xlen {
        key: String,
    },
    Xtrim {
        key: String,
        strategy: String,
        threshold: u64,
        exact: bool,
    },
    Xdel {
        key: String,
        ids: Vec<String>,
    },
    Xread {
        count: Option<u64>,
        keys: Vec<String>,
        ids: Vec<String>,
    },
    // Hash
    Hset {
        key: String,
        fields: Vec<(String, String)>,
    },
    Hget {
        key: String,
        field: String,
    },
    Hdel {
        key: String,
        fields: Vec<String>,
    },
    Hgetall {
        key: String,
    },
    Hexists {
        key: String,
        field: String,
    },
    Hlen {
        key: String,
    },
    Hkeys {
        key: String,
    },
    Hvals {
        key: String,
    },
    // Set
    Sadd {
        key: String,
        members: Vec<String>,
    },
    Smembers {
        key: String,
    },
    Sismember {
        key: String,
        member: String,
    },
    Srem {
        key: String,
        members: Vec<String>,
    },
    Scard {
        key: String,
    },
    // Sorted Set
    Zadd {
        key: String,
        members: Vec<(i64, String)>,
    },
    Zrange {
        key: String,
        start: i64,
        stop: i64,
        withscores: bool,
    },
    Zrank {
        key: String,
        member: String,
    },
    Zscore {
        key: String,
        member: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("ERR wrong number of arguments for '{0}' command")]
    WrongArgCount(String),
    #[error("ERR value is not an integer or out of range")]
    InvalidInteger,
    #[error("ERR syntax error")]
    SyntaxError,
    #[error("ERR unknown command")]
    UnknownCommand,
}

pub fn wrong_arg_count(cmd: &str) -> CmdError {
    CmdError::WrongArgCount(cmd.to_string())
}
