use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, Weak};
use tokio::sync::Notify;

static WAITERS: LazyLock<Mutex<HashMap<String, Vec<Weak<Notify>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// RAII guard that unregisters a BLPOP waiter from all watched keys on drop.
pub struct BlockGuard {
    keys: Vec<String>,
    notify_ptr: usize,
}

impl BlockGuard {
    fn new(keys: Vec<String>, notify: &Arc<Notify>) -> Self {
        Self {
            keys,
            notify_ptr: Arc::as_ptr(notify) as usize,
        }
    }
}

impl Drop for BlockGuard {
    fn drop(&mut self) {
        let mut waiters = WAITERS.lock().unwrap_or_else(|e| e.into_inner());
        for key in &self.keys {
            if let Some(entries) = waiters.get_mut(key) {
                entries.retain(|w| w.as_ptr() as usize != self.notify_ptr);
                if entries.is_empty() {
                    waiters.remove(key);
                }
            }
        }
    }
}

/// Register a Notify for all given keys. Returns a guard that will unregister on drop.
pub fn register(keys: &[String], notify: &Arc<Notify>) -> BlockGuard {
    let mut waiters = WAITERS.lock().unwrap_or_else(|e| e.into_inner());
    for key in keys {
        waiters
            .entry(key.clone())
            .or_default()
            .push(Arc::downgrade(notify));
    }
    BlockGuard::new(keys.to_vec(), notify)
}

/// Notify all waiters for a key.
pub fn notify_waiters(key: &str) {
    let mut waiters = WAITERS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(entries) = waiters.get_mut(key) {
        let mut alive = false;
        for weak in entries.iter() {
            if let Some(notify) = weak.upgrade() {
                notify.notify_one();
                alive = true;
            }
        }
        entries.retain(|w| w.upgrade().is_some());
        if !alive || entries.is_empty() {
            waiters.remove(key);
        }
    }
}
