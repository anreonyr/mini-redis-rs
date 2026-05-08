use bytes::Bytes;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use tokio::time::Instant; // Import Bytes from the byts crate

static DB: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(Bytes),
    List(VecDeque<Bytes>),
    Hash(HashMap<Bytes, Bytes>),
    Set(HashSet<Bytes>),
    ZSet(BTreeSet<(i64, Bytes)>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub value: Value,
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
