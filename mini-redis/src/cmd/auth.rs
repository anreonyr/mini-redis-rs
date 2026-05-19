use crate::config;
use crate::resp::RespType;

/// Per-connection authentication state.
pub struct ConnectionState {
    authenticated: bool,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            authenticated: false,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn authenticate(&mut self) {
        self.authenticated = true;
    }
}

/// Commands that are allowed before authentication.
/// Matches Redis standard: AUTH, PING, ECHO, COMMAND, QUIT, HELLO.
pub fn is_allowed_before_auth(cmd_name: &str) -> bool {
    matches!(
        cmd_name,
        "AUTH" | "PING" | "ECHO" | "COMMAND" | "QUIT" | "HELLO"
    )
}

/// Handle the AUTH command: validate password against requirepass config.
pub fn handle_auth(state: &mut ConnectionState, password: &str) -> RespType {
    let requirepass = config::with_config(|cfg| cfg.requirepass.clone());
    match requirepass {
        Some(ref expected) if expected == password => {
            state.authenticate();
            RespType::SimpleString("OK".to_string())
        }
        Some(_) => RespType::Error("ERR invalid password".to_string()),
        None => RespType::Error(
            "ERR AUTH <password> called without a password configured (set requirepass first)"
                .to_string(),
        ),
    }
}
