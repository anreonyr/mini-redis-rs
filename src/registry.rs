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
}

pub fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&CommandRegistry) -> R,
{
    let reg = REGISTRY.lock().unwrap();
    f(&reg)
}
