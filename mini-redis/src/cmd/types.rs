use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum XGroupSub {
    Create { group: String, id: String },
    CreateConsumer { group: String, consumer: String },
    DelConsumer { group: String, consumer: String },
    Destroy { group: String },
    SetId { group: String, id: String },
}

/// All arguments have been parsed and validated at this point.
#[derive(Clone, Debug, PartialEq)]
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
    Incr {
        key: String,
    },
    Decr {
        key: String,
    },
    Incrby {
        key: String,
        delta: i64,
    },
    Decrby {
        key: String,
        delta: i64,
    },
    Append {
        key: String,
        value: String,
    },
    Strlen {
        key: String,
    },
    Mget {
        keys: Vec<String>,
    },
    Mset {
        pairs: Vec<(String, String)>,
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
    Rpop {
        key: String,
        count: Option<usize>,
    },
    Lindex {
        key: String,
        index: i64,
    },
    Lrem {
        key: String,
        count: i64,
        value: String,
    },
    Ltrim {
        key: String,
        start: i64,
        stop: i64,
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
    Info {
        section: Option<String>,
    },
    ConfigGet {
        parameter: String,
    },
    ConfigSet {
        parameter: String,
        value: String,
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
    // Consumer Groups
    XAck {
        key: String,
        group: String,
        ids: Vec<String>,
    },
    XClaim {
        key: String,
        group: String,
        consumer: String,
        min_idle: u64,
        ids: Vec<String>,
    },
    XGroup {
        sub: XGroupSub,
        key: String,
    },
    XInfo {
        sub: String,
        key: String,
        group: Option<String>,
    },
    XPending {
        key: String,
        group: String,
        start: String,
        end: String,
        count: u64,
        consumer: Option<String>,
    },
    XReadGroup {
        group: String,
        consumer: String,
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
    Spop {
        key: String,
        count: Option<usize>,
    },
    Srandmember {
        key: String,
        count: Option<i64>,
    },
    Sunion {
        keys: Vec<String>,
    },
    Sinter {
        keys: Vec<String>,
    },
    Sdiff {
        keys: Vec<String>,
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
    Zrem {
        key: String,
        members: Vec<String>,
    },
    Zcard {
        key: String,
    },
    Zcount {
        key: String,
        min: String,
        max: String,
    },
    Zrangebyscore {
        key: String,
        min: String,
        max: String,
        withscores: bool,
        limit: Option<(usize, usize)>,
    },
    Zincrby {
        key: String,
        incr: i64,
        member: String,
    },
    Zrevrange {
        key: String,
        start: i64,
        stop: i64,
        withscores: bool,
    },
    Zrevrank {
        key: String,
        member: String,
    },
    // Key Management
    Del {
        keys: Vec<String>,
    },
    Exists {
        keys: Vec<String>,
    },
    Type {
        key: String,
    },
    Keys {
        pattern: String,
    },
    Dbsize,
    // Expiry Management
    Expire {
        key: String,
        seconds: u64,
    },
    Ttl {
        key: String,
    },
    Persist {
        key: String,
    },
    // More String
    Getset {
        key: String,
        value: String,
    },
    Getrange {
        key: String,
        start: i64,
        end: i64,
    },
    Setrange {
        key: String,
        offset: u64,
        value: String,
    },
    Msetnx {
        pairs: Vec<(String, String)>,
    },
    // More List
    Rpoplpush {
        source: String,
        destination: String,
    },
    Lset {
        key: String,
        index: i64,
        value: String,
    },
    // More Hash
    Hincrby {
        key: String,
        field: String,
        incr: i64,
    },
    Hincrbyfloat {
        key: String,
        field: String,
        incr: f64,
    },
    Hsetnx {
        key: String,
        field: String,
        value: String,
    },
    // More Set
    Smove {
        source: String,
        destination: String,
        member: String,
    },
    // More ZSet
    Zremrangebyrank {
        key: String,
        start: i64,
        stop: i64,
    },
    Zremrangebyscore {
        key: String,
        min: String,
        max: String,
    },
    Zrevrangebyscore {
        key: String,
        max: String,
        min: String,
        withscores: bool,
        limit: Option<(usize, usize)>,
    },
    // More Key
    Rename {
        key: String,
        newkey: String,
    },
    Renamenx {
        key: String,
        newkey: String,
    },
    Randomkey,
    // Auth
    Auth {
        password: String,
    },
    Bgsave,
    Save,
    Shutdown,
    // Transaction
    Discard,
    Exec,
    Multi,
    Unwatch,
    Watch {
        keys: Vec<String>,
    },
    // Pub/Sub
    Publish {
        channel: String,
        message: String,
    },
    Subscribe {
        channels: Vec<String>,
    },
    Unsubscribe {
        channels: Vec<String>,
    },
    // Connection management
    ClientGetName,
    ClientSetName {
        name: String,
    },
    Hello,
    Quit,
    Select {
        index: usize,
    },
    // HyperLogLog
    PfAdd {
        key: String,
        elements: Vec<String>,
    },
    PfCount {
        keys: Vec<String>,
    },
    PfMerge {
        dest: String,
        sources: Vec<String>,
    },
}

impl ParsedCmd {
    /// Return the command name as a static string (for auth bypass checks, etc.).
    pub fn name(&self) -> &'static str {
        match self {
            ParsedCmd::Ping => "PING",
            ParsedCmd::Echo { .. } => "ECHO",
            ParsedCmd::Set { .. } => "SET",
            ParsedCmd::Get { .. } => "GET",
            ParsedCmd::Incr { .. } => "INCR",
            ParsedCmd::Decr { .. } => "DECR",
            ParsedCmd::Incrby { .. } => "INCRBY",
            ParsedCmd::Decrby { .. } => "DECRBY",
            ParsedCmd::Append { .. } => "APPEND",
            ParsedCmd::Strlen { .. } => "STRLEN",
            ParsedCmd::Mget { .. } => "MGET",
            ParsedCmd::Mset { .. } => "MSET",
            ParsedCmd::Getset { .. } => "GETSET",
            ParsedCmd::Getrange { .. } => "GETRANGE",
            ParsedCmd::Setrange { .. } => "SETRANGE",
            ParsedCmd::Msetnx { .. } => "MSETNX",
            ParsedCmd::Rpush { .. } => "RPUSH",
            ParsedCmd::Lpush { .. } => "LPUSH",
            ParsedCmd::Lrange { .. } => "LRANGE",
            ParsedCmd::Llen { .. } => "LLEN",
            ParsedCmd::Lpop { .. } => "LPOP",
            ParsedCmd::Rpop { .. } => "RPOP",
            ParsedCmd::Lindex { .. } => "LINDEX",
            ParsedCmd::Lrem { .. } => "LREM",
            ParsedCmd::Ltrim { .. } => "LTRIM",
            ParsedCmd::Rpoplpush { .. } => "RPOPLPUSH",
            ParsedCmd::Lset { .. } => "LSET",
            ParsedCmd::Blpop { .. } => "BLPOP",
            ParsedCmd::Command { .. } => "COMMAND",
            ParsedCmd::Flushdb => "FLUSHDB",
            ParsedCmd::Info { .. } => "INFO",
            ParsedCmd::ConfigGet { .. } => "CONFIG",
            ParsedCmd::ConfigSet { .. } => "CONFIG",
            ParsedCmd::Xadd { .. } => "XADD",
            ParsedCmd::Xrange { .. } => "XRANGE",
            ParsedCmd::Xrevrange { .. } => "XREVRANGE",
            ParsedCmd::Xlen { .. } => "XLEN",
            ParsedCmd::Xtrim { .. } => "XTRIM",
            ParsedCmd::Xdel { .. } => "XDEL",
            ParsedCmd::Xread { .. } => "XREAD",
            ParsedCmd::XAck { .. } => "XACK",
            ParsedCmd::XClaim { .. } => "XCLAIM",
            ParsedCmd::XGroup { .. } => "XGROUP",
            ParsedCmd::XInfo { .. } => "XINFO",
            ParsedCmd::XPending { .. } => "XPENDING",
            ParsedCmd::XReadGroup { .. } => "XREADGROUP",
            ParsedCmd::Hset { .. } => "HSET",
            ParsedCmd::Hget { .. } => "HGET",
            ParsedCmd::Hdel { .. } => "HDEL",
            ParsedCmd::Hgetall { .. } => "HGETALL",
            ParsedCmd::Hexists { .. } => "HEXISTS",
            ParsedCmd::Hlen { .. } => "HLEN",
            ParsedCmd::Hkeys { .. } => "HKEYS",
            ParsedCmd::Hvals { .. } => "HVALS",
            ParsedCmd::Hincrby { .. } => "HINCRBY",
            ParsedCmd::Hincrbyfloat { .. } => "HINCRBYFLOAT",
            ParsedCmd::Hsetnx { .. } => "HSETNX",
            ParsedCmd::Sadd { .. } => "SADD",
            ParsedCmd::Smembers { .. } => "SMEMBERS",
            ParsedCmd::Sismember { .. } => "SISMEMBER",
            ParsedCmd::Srem { .. } => "SREM",
            ParsedCmd::Scard { .. } => "SCARD",
            ParsedCmd::Spop { .. } => "SPOP",
            ParsedCmd::Srandmember { .. } => "SRANDMEMBER",
            ParsedCmd::Sunion { .. } => "SUNION",
            ParsedCmd::Sinter { .. } => "SINTER",
            ParsedCmd::Sdiff { .. } => "SDIFF",
            ParsedCmd::Smove { .. } => "SMOVE",
            ParsedCmd::Zadd { .. } => "ZADD",
            ParsedCmd::Zrange { .. } => "ZRANGE",
            ParsedCmd::Zrank { .. } => "ZRANK",
            ParsedCmd::Zscore { .. } => "ZSCORE",
            ParsedCmd::Zrem { .. } => "ZREM",
            ParsedCmd::Zcard { .. } => "ZCARD",
            ParsedCmd::Zcount { .. } => "ZCOUNT",
            ParsedCmd::Zrangebyscore { .. } => "ZRANGEBYSCORE",
            ParsedCmd::Zincrby { .. } => "ZINCRBY",
            ParsedCmd::Zrevrange { .. } => "ZREVRANGE",
            ParsedCmd::Zrevrank { .. } => "ZREVRANK",
            ParsedCmd::Zremrangebyrank { .. } => "ZREMRANGEBYRANK",
            ParsedCmd::Zremrangebyscore { .. } => "ZREMRANGEBYSCORE",
            ParsedCmd::Zrevrangebyscore { .. } => "ZREVRANGEBYSCORE",
            ParsedCmd::Del { .. } => "DEL",
            ParsedCmd::Exists { .. } => "EXISTS",
            ParsedCmd::Type { .. } => "TYPE",
            ParsedCmd::Keys { .. } => "KEYS",
            ParsedCmd::Dbsize => "DBSIZE",
            ParsedCmd::Expire { .. } => "EXPIRE",
            ParsedCmd::Ttl { .. } => "TTL",
            ParsedCmd::Persist { .. } => "PERSIST",
            ParsedCmd::Rename { .. } => "RENAME",
            ParsedCmd::Renamenx { .. } => "RENAMENX",
            ParsedCmd::Randomkey => "RANDOMKEY",
            ParsedCmd::Auth { .. } => "AUTH",
            ParsedCmd::Bgsave => "BGSAVE",
            ParsedCmd::Save => "SAVE",
            ParsedCmd::Shutdown => "SHUTDOWN",
            ParsedCmd::Discard => "DISCARD",
            ParsedCmd::Exec => "EXEC",
            ParsedCmd::Multi => "MULTI",
            ParsedCmd::Unwatch => "UNWATCH",
            ParsedCmd::Watch { .. } => "WATCH",
            ParsedCmd::Publish { .. } => "PUBLISH",
            ParsedCmd::Subscribe { .. } => "SUBSCRIBE",
            ParsedCmd::Unsubscribe { .. } => "UNSUBSCRIBE",
            ParsedCmd::ClientGetName => "CLIENT",
            ParsedCmd::ClientSetName { .. } => "CLIENT",
            ParsedCmd::Hello => "HELLO",
            ParsedCmd::Quit => "QUIT",
            ParsedCmd::Select { .. } => "SELECT",
            ParsedCmd::PfAdd { .. } => "PFADD",
            ParsedCmd::PfCount { .. } => "PFCOUNT",
            ParsedCmd::PfMerge { .. } => "PFMERGE",
        }
    }
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
