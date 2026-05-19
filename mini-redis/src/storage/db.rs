use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use tokio::time::Instant;

tokio::task_local! {
    /// Per-task database index — no more global mutable atomic across connections.
    pub static DB_INDEX: std::cell::Cell<usize>;
}

pub const NUM_DBS: usize = 16;

/// Per-database Mutex — each DB has its own lock so operations on
/// different databases never contend.
static DBS: LazyLock<Vec<Mutex<HashMap<String, Entry>>>> = LazyLock::new(|| {
    (0..NUM_DBS).map(|_| Mutex::new(HashMap::new())).collect()
});
static VERSION_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Set the current database index for this connection's task.
/// No longer a global atomic — uses `tokio::task_local!` so each
/// spawned connection task has its own independent value.
pub fn set_current_db(index: usize) {
    DB_INDEX.with(|cell| cell.set(index.min(NUM_DBS - 1)));
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamEntry {
    pub id: String,
    pub fields: Vec<(Bytes, Bytes)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamData {
    pub entries: VecDeque<StreamEntry>,
    pub last_timestamp_ms: i64,
    pub last_seq: u64,
    pub groups: HashMap<String, ConsumerGroup>,
}

impl StreamData {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            last_timestamp_ms: 0,
            last_seq: 0,
            groups: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsumerGroup {
    pub name: String,
    pub last_delivered_id: String,
    pub pending: HashMap<String, Vec<PendingEntry>>, // consumer_name → pending entries
    pub consumers: HashMap<String, ConsumerInfo>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingEntry {
    pub id: String,
    pub consumer_name: String,
    pub delivery_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsumerInfo {
    pub name: String,
    pub pending_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(Bytes),
    List(VecDeque<Bytes>),
    Stream(StreamData),
    Hash(HashMap<Bytes, Bytes>),
    Set(HashSet<Bytes>),
    ZSet(BTreeSet<(i64, Bytes)>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub value: Value,
    #[serde(skip)]
    pub expiry: Option<Instant>,
    pub version: u64,
}

impl Entry {
    pub fn new(value: Value, expiry: Option<Instant>) -> Self {
        Self {
            value,
            expiry,
            version: VERSION_COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }
}

pub fn with_db<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, Entry>) -> R,
{
    let idx = DB_INDEX.with(|cell| cell.get());
    let mut db = DBS[idx].lock().unwrap_or_else(|e| e.into_inner());
    f(&mut *db)
}

pub fn with_db_at<F, R>(index: usize, f: F) -> R
where
    F: FnOnce(&mut HashMap<String, Entry>) -> R,
{
    let idx = index.min(NUM_DBS - 1);
    let mut db = DBS[idx].lock().unwrap_or_else(|e| e.into_inner());
    f(&mut *db)
}

pub fn flushdb() {
    for db in DBS.iter() {
        let mut db = db.lock().unwrap_or_else(|e| e.into_inner());
        db.clear();
    }
}

/// Increment and return the next version number.
pub fn bump_version() -> u64 {
    VERSION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Get the current version of a key, or None if the key doesn't exist.
/// Locks the DB internally. Do NOT call from inside `with_db()` (deadlock risk).
pub fn key_version(key: &str) -> Option<u64> {
    let idx = DB_INDEX.with(|cell| cell.get());
    let db = DBS[idx].lock().unwrap_or_else(|e| e.into_inner());
    db.get(key).map(|e| e.version)
}

/// Get a key's version from an already-locked DB reference (safe inside `with_db`).
pub fn entry_version(db: &HashMap<String, Entry>, key: &str) -> Option<u64> {
    db.get(key).map(|e| e.version)
}
