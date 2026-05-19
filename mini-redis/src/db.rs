use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use tokio::time::Instant;

static DB: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

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
}

impl StreamData {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            last_timestamp_ms: 0,
            last_seq: 0,
        }
    }
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
}

impl Entry {
    pub fn new(value: Value, expiry: Option<Instant>) -> Self {
        Self { value, expiry }
    }
}

pub fn with_db<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, Entry>) -> R,
{
    let mut db = DB.lock().unwrap();
    f(&mut db)
}

pub fn flushdb() {
    let mut db = DB.lock().unwrap();
    db.clear();
}
