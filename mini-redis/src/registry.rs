use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub struct CommandInfo {
    pub name: &'static str,
    pub arity: i32,
    pub category: &'static str,
    pub since_stage: u16,
    pub summary: &'static str,
}

pub struct CommandRegistry {
    commands: HashMap<String, CommandInfo>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register(&mut self, info: CommandInfo) {
        self.commands.insert(info.name.to_lowercase(), info);
    }

    pub fn get(&self, name: &str) -> Option<&CommandInfo> {
        self.commands.get(&name.to_lowercase())
    }

    pub fn list_all(&self) -> Vec<&CommandInfo> {
        let mut v: Vec<&CommandInfo> = self.commands.values().collect();
        v.sort_by_key(|c| c.name);
        v
    }

    pub fn exists(&self, name: &str) -> bool {
        self.commands.contains_key(&name.to_lowercase())
    }
}

static REGISTRY: LazyLock<Mutex<CommandRegistry>> =
    LazyLock::new(|| Mutex::new(CommandRegistry::new()));

pub fn init() {
    let mut reg = REGISTRY.lock().unwrap();

    reg.register(CommandInfo {
        name: "PING",
        arity: 1,
        category: "Connection",
        since_stage: 2,
        summary: "Returns PONG if no argument is provided, otherwise returns a copy of the argument",
    });
    reg.register(CommandInfo {
        name: "ECHO",
        arity: 2,
        category: "Connection",
        since_stage: 5,
        summary: "Returns the given string",
    });
    reg.register(CommandInfo {
        name: "SET",
        arity: -3,
        category: "String",
        since_stage: 6,
        summary: "Sets the string value of a key, optionally with EX/PX expiry",
    });
    reg.register(CommandInfo {
        name: "GET",
        arity: 2,
        category: "String",
        since_stage: 6,
        summary: "Gets the string value of a key, or nil when key does not exist",
    });
    reg.register(CommandInfo {
        name: "INCR",
        arity: 2,
        category: "String",
        since_stage: 0,
        summary: "Increments the integer value of a key by one",
    });
    reg.register(CommandInfo {
        name: "DECR",
        arity: 2,
        category: "String",
        since_stage: 0,
        summary: "Decrements the integer value of a key by one",
    });
    reg.register(CommandInfo {
        name: "INCRBY",
        arity: 3,
        category: "String",
        since_stage: 0,
        summary: "Increments the integer value of a key by a given amount",
    });
    reg.register(CommandInfo {
        name: "DECRBY",
        arity: 3,
        category: "String",
        since_stage: 0,
        summary: "Decrements the integer value of a key by a given amount",
    });
    reg.register(CommandInfo {
        name: "APPEND",
        arity: 3,
        category: "String",
        since_stage: 0,
        summary: "Appends a value to a key",
    });
    reg.register(CommandInfo {
        name: "STRLEN",
        arity: 2,
        category: "String",
        since_stage: 0,
        summary: "Returns the length of the string value of a key",
    });
    reg.register(CommandInfo {
        name: "MGET",
        arity: -2,
        category: "String",
        since_stage: 0,
        summary: "Gets the values of all the given keys",
    });
    reg.register(CommandInfo {
        name: "MSET",
        arity: -3,
        category: "String",
        since_stage: 0,
        summary: "Sets multiple keys to multiple values",
    });
    reg.register(CommandInfo {
        name: "RPUSH",
        arity: -3,
        category: "List",
        since_stage: 9,
        summary: "Appends one or more elements to the right of a list",
    });
    reg.register(CommandInfo {
        name: "LPUSH",
        arity: -3,
        category: "List",
        since_stage: 13,
        summary: "Prepends one or more elements to the left of a list",
    });
    reg.register(CommandInfo {
        name: "LRANGE",
        arity: 4,
        category: "List",
        since_stage: 11,
        summary: "Returns a range of elements from a list",
    });
    reg.register(CommandInfo {
        name: "LLEN",
        arity: 2,
        category: "List",
        since_stage: 14,
        summary: "Returns the length of a list",
    });
    reg.register(CommandInfo {
        name: "LPOP",
        arity: -2,
        category: "List",
        since_stage: 15,
        summary: "Removes and returns the first element(s) of a list",
    });
    reg.register(CommandInfo {
        name: "RPOP",
        arity: -2,
        category: "List",
        since_stage: 0,
        summary: "Removes and returns the last element(s) of a list",
    });
    reg.register(CommandInfo {
        name: "LINDEX",
        arity: 3,
        category: "List",
        since_stage: 0,
        summary: "Returns an element from a list by its index",
    });
    reg.register(CommandInfo {
        name: "LREM",
        arity: 4,
        category: "List",
        since_stage: 0,
        summary: "Removes elements from a list by value",
    });
    reg.register(CommandInfo {
        name: "LTRIM",
        arity: 4,
        category: "List",
        since_stage: 0,
        summary: "Trims a list to the specified range",
    });
    reg.register(CommandInfo {
        name: "BLPOP",
        arity: -3,
        category: "List",
        since_stage: 17,
        summary: "Removes and returns the first element of a list; blocks on empty lists with timeout",
    });
    reg.register(CommandInfo {
        name: "COMMAND",
        arity: -1,
        category: "Server",
        since_stage: 0,
        summary: "Returns information about all commands or a specific command",
    });
    reg.register(CommandInfo {
        name: "FLUSHDB",
        arity: 1,
        category: "Server",
        since_stage: 0,
        summary: "Removes all data from the current database",
    });
    reg.register(CommandInfo {
        name: "INFO",
        arity: -1,
        category: "Server",
        since_stage: 0,
        summary: "Returns information about the server",
    });
    reg.register(CommandInfo {
        name: "CONFIG",
        arity: -2,
        category: "Server",
        since_stage: 0,
        summary: "Gets configuration parameters",
    });
    // Streams
    reg.register(CommandInfo {
        name: "XADD",
        arity: -4,
        category: "Stream",
        since_stage: 0,
        summary: "Appends a new entry to a stream",
    });
    reg.register(CommandInfo {
        name: "XRANGE",
        arity: -4,
        category: "Stream",
        since_stage: 0,
        summary: "Returns a range of entries from a stream",
    });
    reg.register(CommandInfo {
        name: "XREVRANGE",
        arity: -4,
        category: "Stream",
        since_stage: 0,
        summary: "Returns a range of entries in reverse order",
    });
    reg.register(CommandInfo {
        name: "XLEN",
        arity: 2,
        category: "Stream",
        since_stage: 0,
        summary: "Returns the length of a stream",
    });
    reg.register(CommandInfo {
        name: "XTRIM",
        arity: -3,
        category: "Stream",
        since_stage: 0,
        summary: "Trims a stream to a given length",
    });
    reg.register(CommandInfo {
        name: "XDEL",
        arity: -3,
        category: "Stream",
        since_stage: 0,
        summary: "Removes one or more entries from a stream",
    });
    reg.register(CommandInfo {
        name: "XREAD",
        arity: -4,
        category: "Stream",
        since_stage: 0,
        summary: "Reads data from one or more streams",
    });
    // Key Management
    reg.register(CommandInfo {
        name: "DEL",
        arity: -2,
        category: "Generic",
        since_stage: 0,
        summary: "Deletes one or more keys",
    });
    reg.register(CommandInfo {
        name: "EXISTS",
        arity: -2,
        category: "Generic",
        since_stage: 0,
        summary: "Determines whether one or more keys exist",
    });
    reg.register(CommandInfo {
        name: "TYPE",
        arity: 2,
        category: "Generic",
        since_stage: 0,
        summary: "Returns the type of a key",
    });
    reg.register(CommandInfo {
        name: "KEYS",
        arity: 2,
        category: "Generic",
        since_stage: 0,
        summary: "Finds all keys matching a pattern",
    });
    reg.register(CommandInfo {
        name: "DBSIZE",
        arity: 1,
        category: "Server",
        since_stage: 0,
        summary: "Returns the number of keys in the database",
    });
    // Expiry Management
    reg.register(CommandInfo {
        name: "EXPIRE",
        arity: 3,
        category: "Generic",
        since_stage: 0,
        summary: "Sets a key's time to live in seconds",
    });
    reg.register(CommandInfo {
        name: "TTL",
        arity: 2,
        category: "Generic",
        since_stage: 0,
        summary: "Gets the remaining time to live of a key in seconds",
    });
    reg.register(CommandInfo {
        name: "PERSIST",
        arity: 2,
        category: "Generic",
        since_stage: 0,
        summary: "Removes the expiration from a key",
    });
    // More String
    reg.register(CommandInfo {
        name: "GETSET",
        arity: 3,
        category: "String",
        since_stage: 0,
        summary: "Sets the string value and returns its old value",
    });
    reg.register(CommandInfo {
        name: "GETRANGE",
        arity: 4,
        category: "String",
        since_stage: 0,
        summary: "Returns a substring of the string value",
    });
    reg.register(CommandInfo {
        name: "SETRANGE",
        arity: 4,
        category: "String",
        since_stage: 0,
        summary: "Overwrites part of a string at the given offset",
    });
    reg.register(CommandInfo {
        name: "MSETNX",
        arity: -3,
        category: "String",
        since_stage: 0,
        summary: "Sets multiple keys to multiple values, only if none exist",
    });
    // More List
    reg.register(CommandInfo {
        name: "RPOPLPUSH",
        arity: 3,
        category: "List",
        since_stage: 0,
        summary: "Pops an element from a list and pushes it to another",
    });
    reg.register(CommandInfo {
        name: "LSET",
        arity: 4,
        category: "List",
        since_stage: 0,
        summary: "Sets the value of an element in a list by its index",
    });
    // More Hash
    reg.register(CommandInfo {
        name: "HINCRBY",
        arity: 4,
        category: "Hash",
        since_stage: 0,
        summary: "Increments the integer value of a hash field",
    });
    reg.register(CommandInfo {
        name: "HINCRBYFLOAT",
        arity: 4,
        category: "Hash",
        since_stage: 0,
        summary: "Increments the float value of a hash field",
    });
    reg.register(CommandInfo {
        name: "HSETNX",
        arity: 4,
        category: "Hash",
        since_stage: 0,
        summary: "Sets the value of a hash field, only if the field does not exist",
    });
    // More Set
    reg.register(CommandInfo {
        name: "SMOVE",
        arity: 4,
        category: "Set",
        since_stage: 0,
        summary: "Moves a member from one set to another",
    });
    // More ZSet
    reg.register(CommandInfo {
        name: "ZREMRANGEBYRANK",
        arity: 4,
        category: "ZSet",
        since_stage: 0,
        summary: "Removes all members in a sorted set within the given rank range",
    });
    reg.register(CommandInfo {
        name: "ZREMRANGEBYSCORE",
        arity: 4,
        category: "ZSet",
        since_stage: 0,
        summary: "Removes all members in a sorted set within the given score range",
    });
    reg.register(CommandInfo {
        name: "ZREVRANGEBYSCORE",
        arity: -3,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns a range of members in a sorted set, by score, in reverse order",
    });
    // Auth
    reg.register(CommandInfo {
        name: "AUTH",
        arity: 2,
        category: "Connection",
        since_stage: 0,
        summary: "Authenticates the connection using a password",
    });
    reg.register(CommandInfo {
        name: "PUBLISH",
        arity: 3,
        category: "PubSub",
        since_stage: 0,
        summary: "Posts a message to a channel",
    });
    reg.register(CommandInfo {
        name: "SUBSCRIBE",
        arity: -2,
        category: "PubSub",
        since_stage: 0,
        summary: "Subscribes to one or more channels",
    });
    reg.register(CommandInfo {
        name: "UNSUBSCRIBE",
        arity: -1,
        category: "PubSub",
        since_stage: 0,
        summary: "Unsubscribes from one or more channels",
    });
    reg.register(CommandInfo {
        name: "SAVE",
        arity: 1,
        category: "Server",
        since_stage: 0,
        summary: "Synchronously saves the dataset to disk",
    });
    reg.register(CommandInfo {
        name: "BGSAVE",
        arity: 1,
        category: "Server",
        since_stage: 0,
        summary: "Asynchronously saves the dataset to disk in background",
    });
    reg.register(CommandInfo {
        name: "SHUTDOWN",
        arity: 1,
        category: "Server",
        since_stage: 0,
        summary: "Synchronously saves the dataset to disk and shuts down",
    });
    // More Key
    reg.register(CommandInfo {
        name: "RENAME",
        arity: 3,
        category: "Generic",
        since_stage: 0,
        summary: "Renames a key",
    });
    reg.register(CommandInfo {
        name: "RENAMENX",
        arity: 3,
        category: "Generic",
        since_stage: 0,
        summary: "Renames a key, only if the new key does not exist",
    });
    reg.register(CommandInfo {
        name: "RANDOMKEY",
        arity: 1,
        category: "Generic",
        since_stage: 0,
        summary: "Returns a random key name from the database",
    });
    // Hash
    reg.register(CommandInfo {
        name: "HSET",
        arity: -4,
        category: "Hash",
        since_stage: 0,
        summary: "Sets field(s) in a hash",
    });
    reg.register(CommandInfo {
        name: "HGET",
        arity: 3,
        category: "Hash",
        since_stage: 0,
        summary: "Returns the value of a hash field",
    });
    reg.register(CommandInfo {
        name: "HDEL",
        arity: -3,
        category: "Hash",
        since_stage: 0,
        summary: "Deletes one or more hash fields",
    });
    reg.register(CommandInfo {
        name: "HGETALL",
        arity: 2,
        category: "Hash",
        since_stage: 0,
        summary: "Returns all fields and values of a hash",
    });
    reg.register(CommandInfo {
        name: "HEXISTS",
        arity: 3,
        category: "Hash",
        since_stage: 0,
        summary: "Determines whether a hash field exists",
    });
    reg.register(CommandInfo {
        name: "HLEN",
        arity: 2,
        category: "Hash",
        since_stage: 0,
        summary: "Returns the number of fields in a hash",
    });
    reg.register(CommandInfo {
        name: "HKEYS",
        arity: 2,
        category: "Hash",
        since_stage: 0,
        summary: "Returns all field names in a hash",
    });
    reg.register(CommandInfo {
        name: "HVALS",
        arity: 2,
        category: "Hash",
        since_stage: 0,
        summary: "Returns all values in a hash",
    });
    // Set
    reg.register(CommandInfo {
        name: "SADD",
        arity: -3,
        category: "Set",
        since_stage: 0,
        summary: "Adds one or more members to a set",
    });
    reg.register(CommandInfo {
        name: "SMEMBERS",
        arity: 2,
        category: "Set",
        since_stage: 0,
        summary: "Returns all members of a set",
    });
    reg.register(CommandInfo {
        name: "SISMEMBER",
        arity: 3,
        category: "Set",
        since_stage: 0,
        summary: "Determines whether a value is a member of a set",
    });
    reg.register(CommandInfo {
        name: "SREM",
        arity: -3,
        category: "Set",
        since_stage: 0,
        summary: "Removes one or more members from a set",
    });
    reg.register(CommandInfo {
        name: "SCARD",
        arity: 2,
        category: "Set",
        since_stage: 0,
        summary: "Returns the cardinality of a set",
    });
    reg.register(CommandInfo {
        name: "SPOP",
        arity: -2,
        category: "Set",
        since_stage: 0,
        summary: "Removes and returns random members from a set",
    });
    reg.register(CommandInfo {
        name: "SRANDMEMBER",
        arity: -2,
        category: "Set",
        since_stage: 0,
        summary: "Returns random members from a set",
    });
    reg.register(CommandInfo {
        name: "SUNION",
        arity: -2,
        category: "Set",
        since_stage: 0,
        summary: "Returns the union of multiple sets",
    });
    reg.register(CommandInfo {
        name: "SINTER",
        arity: -2,
        category: "Set",
        since_stage: 0,
        summary: "Returns the intersection of multiple sets",
    });
    reg.register(CommandInfo {
        name: "SDIFF",
        arity: -2,
        category: "Set",
        since_stage: 0,
        summary: "Returns the difference of multiple sets",
    });
    // Sorted Set
    reg.register(CommandInfo {
        name: "ZADD",
        arity: -4,
        category: "ZSet",
        since_stage: 0,
        summary: "Adds one or more members to a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZRANGE",
        arity: -4,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns a range of members in a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZRANK",
        arity: 3,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns the rank of a member in a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZSCORE",
        arity: 3,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns the score of a member in a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZREM",
        arity: -3,
        category: "ZSet",
        since_stage: 0,
        summary: "Removes one or more members from a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZCARD",
        arity: 2,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns the number of members in a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZCOUNT",
        arity: 4,
        category: "ZSet",
        since_stage: 0,
        summary: "Counts the members in a sorted set with scores within a range",
    });
    reg.register(CommandInfo {
        name: "ZRANGEBYSCORE",
        arity: -3,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns a range of members in a sorted set by score",
    });
    reg.register(CommandInfo {
        name: "ZINCRBY",
        arity: 4,
        category: "ZSet",
        since_stage: 0,
        summary: "Increments the score of a member in a sorted set",
    });
    reg.register(CommandInfo {
        name: "ZREVRANGE",
        arity: -3,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns a range of members in a sorted set in reverse order",
    });
    reg.register(CommandInfo {
        name: "ZREVRANK",
        arity: 3,
        category: "ZSet",
        since_stage: 0,
        summary: "Returns the rank of a member in a sorted set, ordered high to low",
    });
    // Transaction
    reg.register(CommandInfo {
        name: "MULTI",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Marks the start of a transaction block",
    });
    reg.register(CommandInfo {
        name: "EXEC",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Executes all commands in a transaction block",
    });
    reg.register(CommandInfo {
        name: "DISCARD",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Discards all commands in a transaction block",
    });
    reg.register(CommandInfo {
        name: "WATCH",
        arity: -2,
        category: "Transaction",
        since_stage: 0,
        summary: "Watches one or more keys for changes",
    });
    reg.register(CommandInfo {
        name: "UNWATCH",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Forgets all watched keys",
    });
}

pub fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&CommandRegistry) -> R,
{
    let reg = REGISTRY.lock().unwrap();
    f(&reg)
}
