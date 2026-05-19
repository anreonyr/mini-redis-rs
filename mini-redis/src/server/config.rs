use std::sync::{LazyLock, Mutex};

/// Server-level configuration: authentication password, RDB directory and filename.
pub struct ServerConfig {
    pub requirepass: Option<String>,
    pub dir: String,
    pub dbfilename: String,
}

impl ServerConfig {
    pub fn new() -> Self {
        let password = std::env::var("REDIS_PASSWORD").ok().filter(|s| !s.is_empty());
        Self {
            requirepass: password,
            dir: ".".to_string(),
            dbfilename: "dump.db".to_string(),
        }
    }

    pub fn requirepass_is_set(&self) -> bool {
        self.requirepass.is_some()
    }

    pub fn db_path(&self) -> String {
        format!("{}/{}", self.dir.trim_end_matches('/').trim_end_matches('\\'), self.dbfilename)
    }
}

static CONFIG: LazyLock<Mutex<ServerConfig>> =
    LazyLock::new(|| Mutex::new(ServerConfig::new()));

pub fn with_config<F, R>(f: F) -> R
where
    F: FnOnce(&ServerConfig) -> R,
{
    let cfg = CONFIG.lock().unwrap();
    f(&cfg)
}

pub fn with_config_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut ServerConfig) -> R,
{
    let mut cfg = CONFIG.lock().unwrap();
    f(&mut cfg)
}

/// Override requirepass from CLI args (called once at startup).
pub fn set_requirepass_from_cli(password: String) {
    let mut cfg = CONFIG.lock().unwrap();
    cfg.requirepass = Some(password);
}
