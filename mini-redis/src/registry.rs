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
}

pub fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&CommandRegistry) -> R,
{
    let reg = REGISTRY.lock().unwrap();
    f(&reg)
}
