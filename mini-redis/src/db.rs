use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};
use tokio::time::Instant;

pub const NUM_DBS: usize = 16;

static DBS: LazyLock<Mutex<Vec<HashMap<String, Entry>>>> = LazyLock::new(|| {
    let mut vec = Vec::with_capacity(NUM_DBS);
    for _ in 0..NUM_DBS {
        vec.push(HashMap::new());
    }
    Mutex::new(vec)
});
static VERSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static CURRENT_DB: AtomicUsize = AtomicUsize::new(0);

/// Set the current database index (used by dispatch_command before calling handlers).
pub fn set_current_db(index: usize) {
    CURRENT_DB.store(index.min(NUM_DBS - 1), Ordering::Relaxed);
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
    HyperLogLog(Vec<u8>),
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
    let idx = CURRENT_DB.load(Ordering::Relaxed);
    let mut dbs = DBS.lock().unwrap();
    f(&mut dbs[idx])
}

pub fn with_db_at<F, R>(index: usize, f: F) -> R
where
    F: FnOnce(&mut HashMap<String, Entry>) -> R,
{
    let mut dbs = DBS.lock().unwrap();
    let idx = index.min(dbs.len() - 1);
    f(&mut dbs[idx])
}

pub fn flushdb() {
    let mut dbs = DBS.lock().unwrap();
    for db in dbs.iter_mut() {
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
    let idx = CURRENT_DB.load(Ordering::Relaxed);
    let dbs = DBS.lock().unwrap();
    dbs[idx].get(key).map(|e| e.version)
}

/// Get a key's version from an already-locked DB reference (safe inside `with_db`).
pub fn entry_version(db: &HashMap<String, Entry>, key: &str) -> Option<u64> {
    db.get(key).map(|e| e.version)
}
