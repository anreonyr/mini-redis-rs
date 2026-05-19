use std::collections::HashMap;

use crate::cmd::types::ParsedCmd;
use crate::config;
use crate::pubsub::Message;
use crate::resp::RespType;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Clone)]
pub struct TransactionState {
    pub queue: Vec<ParsedCmd>,
}

impl TransactionState {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
        }
    }
}

/// Per-connection subscription state (only set in subscription mode).
pub struct SubscriptionState {
    pub rx: UnboundedReceiver<Message>,
}

/// Per-connection authentication and transaction state.
pub struct ConnectionState {
    authenticated: bool,
    pub transaction: Option<TransactionState>,
    pub watching: HashMap<String, u64>,
    pub subscription: Option<SubscriptionState>,
    pub db_index: usize,
    pub quit: bool,
    pub client_name: Option<String>,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            authenticated: false,
            transaction: None,
            watching: HashMap::new(),
            subscription: None,
            db_index: 0,
            quit: false,
            client_name: None,
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.subscription.is_some()
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
