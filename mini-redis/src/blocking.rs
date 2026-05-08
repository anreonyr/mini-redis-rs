use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Weak};
use tokio::sync::Notify;

static WAITERS: LazyLock<Mutex<HashMap<String, Vec<(u64, Weak<Notify>)>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct BlpopGuard {
    keys: Vec<String>,
    id: u64,
}

impl BlpopGuard {
    fn new(keys: Vec<String>, id: u64) -> Self {
        Self { keys, id }
    }
}

impl Drop for BlpopGuard {
    fn drop(&mut self) {
        if self.id == 0 {
            return;
        }
        let mut waiters = WAITERS.lock().unwrap();
        for key in &self.keys {
            if let Some(entries) = waiters.get_mut(key) {
                entries.retain(|(id, _)| *id != self.id);
                if entries.is_empty() {
                    waiters.remove(key);
                }
            }
        }
    }
}

/// Register a Notify for all given keys. Returns a guard that will unregister on drop.
pub fn register(keys: &[String], notify: &Arc<Notify>) -> BlpopGuard {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut waiters = WAITERS.lock().unwrap();
    for key in keys {
        waiters
            .entry(key.clone())
            .or_default()
            .push((id, Arc::downgrade(notify)));
    }
    BlpopGuard::new(keys.to_vec(), id)
}

/// Notify all waiters for a key.
pub fn notify_waiters(key: &str) {
    let mut waiters = WAITERS.lock().unwrap();
    if let Some(entries) = waiters.get_mut(key) {
        // Collect which entries to keep (failed upgrades are stale)
        let mut alive = false;
        for (_, weak) in entries.iter() {
            if let Some(notify) = weak.upgrade() {
                notify.notify_one();
                alive = true;
            }
        }
        // Remove stale entries kept alive by dead waiters
        entries.retain(|(_, weak)| weak.upgrade().is_some());
        if !alive || entries.is_empty() {
            waiters.remove(key);
        }
    }
}
