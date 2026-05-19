# 事务功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 mini-redis 添加完整事务支持：MULTI/EXEC/DISCARD/WATCH/UNWATCH

**Architecture:** 每个连接维护 `TransactionState`（命令队列 + 监视 key 版本快照）。`db.rs` 中全局版本计数器，每次写操作 bump 版本号。EXEC 时对比 WATCH 记录的版本号实现乐观锁。

**Tech Stack:** Rust, tokio, AtomicU64

---

### Task 1: 版本号机制 + TransactionState

**Files:**
- Modify: `mini-redis/src/db.rs`
- Modify: `mini-redis/src/cmd/auth.rs`

- [ ] **Step 1: db.rs — 全局版本计数器 + Entry 加 version 字段**

在 `db.rs` 顶部添加：

```rust
use std::sync::atomic::{AtomicU64, Ordering};

static VERSION_COUNTER: AtomicU64 = AtomicU64::new(1);
```

在 `Entry` 结构体添加 `version: u64` 字段：

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub value: Value,
    #[serde(skip)]
    pub expiry: Option<Instant>,
    pub version: u64,
}
```

更新 `Entry::new()`（需要添加 `version` 参数或内部直接 `bump_version()`）：

方式一（推荐）：内部 bump，调用者不需要传：

```rust
impl Entry {
    pub fn new(value: Value, expiry: Option<Instant>) -> Self {
        Self {
            value,
            expiry,
            version: VERSION_COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }
}
```

在 `db.rs` 添加公共函数：

```rust
/// Increment and return the next version number.
pub fn bump_version() -> u64 {
    VERSION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Get the current version of a key, or None if the key doesn't exist.
pub fn key_version(key: &str) -> Option<u64> {
    let db = DB.lock().unwrap();
    db.get(key).map(|e| e.version)
}
```

注意：`bump_version()` 在持锁外调用（先 bump 再写 db），避免死锁。

- [ ] **Step 2: auth.rs — ConnectionState 加 TransactionState**

在 `mini-redis/src/cmd/auth.rs` 中添加事务相关类型和状态：

```rust
use crate::cmd::types::ParsedCmd;
use std::collections::HashMap;

/// Holds the state for a connection's current transaction.
#[derive(Clone)]
pub struct TransactionState {
    pub queue: Vec<ParsedCmd>,
    pub watching: HashMap<String, u64>, // key → version at watch time
}

impl TransactionState {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            watching: HashMap::new(),
        }
    }
}
```

在 `ConnectionState` 添加 `transaction` 字段：

```rust
pub struct ConnectionState {
    authenticated: bool,
    pub transaction: Option<TransactionState>,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            authenticated: false,
            transaction: None,
        }
    }
    // ...保留现有方法...
}
```

注意：`TransactionState` 会 `Clone`，因为 handler 需要 move 它，但在 dispatch 时需要修改。后面 Task 2 处理这个问题。

- [ ] **Step 3: 编译验证**

```bash
cargo build --release
```
Expected: 编译有少量错误（dispatch.rs 等未加新命令的 match），但 db.rs 和 auth.rs 部分通过

- [ ] **Step 4: Commit**

```bash
git add mini-redis/src/db.rs mini-redis/src/cmd/auth.rs
git commit -m "feat: add version counter and TransactionState for transactions"
```

---

### Task 2: MULTI/EXEC/DISCARD/WATCH/UNWATCH 命令

**Files:**
- Modify: `mini-redis/src/cmd/types.rs`
- Modify: `mini-redis/src/cmd/parse.rs`
- Modify: `mini-redis/src/cmd/dispatch.rs`
- Modify: `mini-redis/src/cmd/handlers/connection.rs`
- Modify: `mini-redis/src/registry.rs`

- [ ] **Step 1: types.rs — 添加新命令变体**

在 `ParsedCmd` 枚举中添加：

```rust
    Discard,
    Exec,
    Multi,
    Unwatch,
    Watch {
        keys: Vec<String>,
    },
```

在 `ParsedCmd::name()` 中添加：

```rust
    ParsedCmd::Discard => "DISCARD",
    ParsedCmd::Exec => "EXEC",
    ParsedCmd::Multi => "MULTI",
    ParsedCmd::Unwatch => "UNWATCH",
    ParsedCmd::Watch { .. } => "WATCH",
```

- [ ] **Step 2: parse.rs — 添加解析**

在 `ParsedCmd::parse()` 的 `match cmd` 中添加：

```rust
"MULTI" => ParsedCmd::Multi,
"EXEC" => ParsedCmd::Exec,
"DISCARD" => ParsedCmd::Discard,
"UNWATCH" => ParsedCmd::Unwatch,
"WATCH" => {
    if args.is_empty() {
        return Err(wrong_arg_count("watch"));
    }
    ParsedCmd::Watch { keys: args }
}
```

- [ ] **Step 3: registry.rs — 注册命令**

```rust
    reg.register(CommandInfo {
        name: "MULTI",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Marks the start of a transaction block",
    });
    reg.register(CommandInfo {
        name: "EXEC",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Executes all commands in a transaction block",
    });
    reg.register(CommandInfo {
        name: "DISCARD",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Discards all commands in a transaction block",
    });
    reg.register(CommandInfo {
        name: "WATCH",
        arity: -2,
        category: "Transaction",
        since_stage: 0,
        summary: "Watches one or more keys for changes",
    });
    reg.register(CommandInfo {
        name: "UNWATCH",
        arity: 1,
        category: "Transaction",
        since_stage: 0,
        summary: "Forgets all watched keys",
    });
```

- [ ] **Step 4: dispatch.rs — 路由**

在 `match parsed { ... }` 中添加：

```rust
    ParsedCmd::Multi => handlers::handle_multi(state),
    ParsedCmd::Exec => handlers::handle_exec(state).await,
    ParsedCmd::Discard => handlers::handle_discard(state),
    ParsedCmd::Watch { keys } => handlers::handle_watch(state, &keys),
    ParsedCmd::Unwatch => handlers::handle_unwatch(state),
```

注意：`handle_exec` 需要是 async 的，因为执行队列中的命令需要异步执行（比如 BLPOP）。但其他 handler 是同步的。

dispatch.rs 中 `dispatch_command` 已经是 async 的，所以可以调用 async handler。

- [ ] **Step 5: connection.rs — handler 实现**

在 `connection.rs` 中添加导入：

```rust
use crate::cmd::auth::{ConnectionState, TransactionState};
use crate::db;
use crate::resp::RespType;
use std::collections::HashMap;
```

添加 handler 函数：

```rust
pub fn handle_multi(state: &mut ConnectionState) -> RespType {
    if state.transaction.is_some() {
        return RespType::Error("ERR MULTI calls can not be nested".to_string());
    }
    state.transaction = Some(TransactionState::new());
    RespType::SimpleString("OK".to_string())
}

pub async fn handle_exec(state: &mut ConnectionState) -> RespType {
    let tx = match state.transaction.take() {
        Some(tx) => tx,
        None => return RespType::Error("ERR EXEC without MULTI".to_string()),
    };

    // Check watched keys
    for (key, recorded_version) in &tx.watching {
        if db::key_version(key) != Some(*recorded_version) {
            // Key changed — transaction aborted
            return RespType::Array(None); // nil array
        }
    }

    // Execute queue
    let mut results = Vec::with_capacity(tx.queue.len());
    for cmd in tx.queue {
        // Re-dispatch each command — but without the transaction context
        let response = crate::cmd::dispatch_command(Ok(cmd), state).await;
        results.push(response);
    }

    RespType::Array(Some(results))
}

pub fn handle_discard(state: &mut ConnectionState) -> RespType {
    if state.transaction.is_none() {
        return RespType::Error("ERR DISCARD without MULTI".to_string());
    }
    state.transaction = None;
    RespType::SimpleString("OK".to_string())
}

pub fn handle_watch(state: &mut ConnectionState, keys: &[String]) -> RespType {
    let versions: HashMap<String, u64> = keys
        .iter()
        .map(|k| (k.clone(), db::key_version(k).unwrap_or(0)))
        .collect();

    if let Some(tx) = &mut state.transaction {
        tx.watching.extend(versions);
    } else {
        // WATCH outside MULTI — create a temporary watching state
        // (Redis allows WATCH outside MULTI)
        if state.transaction.is_none() {
            let mut tx = TransactionState::new();
            tx.watching = versions;
            state.transaction = Some(tx);
        }
    }
    RespType::SimpleString("OK".to_string())
}

pub fn handle_unwatch(state: &mut ConnectionState) -> RespType {
    if let Some(tx) = &mut state.transaction {
        tx.watching.clear();
    }
    RespType::SimpleString("OK".to_string())
}
```

注意：
- EXEC 的 `take()` 后需要重新 dispatch 队列中的命令，这些命令会递归调用 `dispatch_command`。为避免无限递归，需要在 `dispatch_command` 中检测 EXEC 不会入队（已在入队规则中保证）。
- WATCH 在事务外也支持（Redis 行为：WATCH 后再 MULTI）。

- [ ] **Step 6: dispatch.rs — 修改 dispatch 支持事务入队**

在 `dispatch_command` 中，在 auth 检查之后、`match parsed` 之前，添加事务入队逻辑：

```rust
// Transaction queueing: if in a transaction and command is queueable
if let Some(ref mut tx) = state.transaction {
    let bypass = matches!(&parsed,
        ParsedCmd::Multi | ParsedCmd::Exec
        | ParsedCmd::Discard | ParsedCmd::Watch { .. }
        | ParsedCmd::Unwatch
    );
    if !bypass {
        tx.queue.push(parsed);
        return RespType::SimpleString("QUEUED".to_string());
    }
}
```

注意：这段代码放在 `match parsed { ... }` 之前。

- [ ] **Step 7: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 8: Commit**

```bash
git add mini-redis/src/cmd/types.rs mini-redis/src/cmd/parse.rs mini-redis/src/cmd/dispatch.rs mini-redis/src/cmd/handlers/connection.rs mini-redis/src/registry.rs
git commit -m "feat: add MULTI/EXEC/DISCARD/WATCH/UNWATCH commands"
```

---

### Task 3: 写操作调用 bump_version()

**Files:**
- Modify: `mini-redis/src/cmd/handlers/string.rs`
- Modify: `mini-redis/src/cmd/handlers/key.rs`
- Modify: `mini-redis/src/cmd/handlers/list.rs`
- Modify: `mini-redis/src/cmd/handlers/set.rs`
- Modify: `mini-redis/src/cmd/handlers/hash.rs`
- Modify: `mini-redis/src/cmd/handlers/zset.rs`
- Modify: `mini-redis/src/cmd/handlers/stream.rs`
- Modify: `mini-redis/src/cmd/handlers/connection.rs` (FLUSHDB)

这个任务比较机械，每个 handler 在创建或修改 key 时调用 `db::bump_version()`。

**规则：** 任何修改 key value 的操作后调用 `db::bump_version()`。

- [ ] **Step 1: string.rs — 添加 bump_version() 调用**

检查每个写 handler，在修改数据后添加 `db::bump_version();`。例如：

`handle_set` 中 `with_db` 闭包内修改完 Entry 后（或在闭包外，每次 SET 成功后就 bump）：

```rust
pub fn handle_set(key: &str, value: &str, expiry: Option<Duration>) -> RespType {
    // ... existing logic ...
    db::bump_version(); // <-- add this after successful set
    RespType::SimpleString("OK".to_string())
}
```

需要添加 `use crate::db;` 如果还没有的话。

受影响的 string.rs handler：
- handle_set ✅
- handle_incr ✅
- handle_decr ✅
- handle_incrby ✅
- handle_decrby ✅
- handle_append ✅
- handle_mset ✅
- handle_getset ✅
- handle_setrange ✅
- handle_msetnx ✅

- [ ] **Step 2: key.rs — 添加 bump_version()**

受影响的 key.rs handler：
- handle_del ✅
- handle_rename ✅ (rename modifies the key; bump version for both old and new key)
- handle_renamenx ✅

- [ ] **Step 2b: expiry.rs — 添加 bump_version()**

`handle_expire` modifies `entry.expiry` which changes the key's state — WATCH should detect this:

```rust
pub fn handle_expire(key: &str, seconds: u64) -> RespType {
    let found = db::with_db(|db| {
        if let Some(entry) = db.get_mut(key) {
            entry.expiry = Some(Instant::now() + Duration::from_secs(seconds));
            entry.version = db::bump_version();
            true
        } else {
            false
        }
    });
    // ...rest unchanged
}
```

Add `use crate::db;` and `use std::time::Duration;` if not already imported in expiry.rs.

- [ ] **Step 3: list.rs — 添加 bump_version()**

受影响的 list.rs handler：
- handle_rpush ✅
- handle_lpush ✅
- handle_lpop ✅
- handle_rpop ✅
- handle_lrem ✅
- handle_ltrim ✅
- handle_rpoplpush ✅
- handle_lset ✅

- [ ] **Step 4: set.rs — 添加 bump_version()**

受影响的 set.rs handler：
- handle_sadd ✅
- handle_srem ✅
- handle_spop ✅
- handle_smove ✅

- [ ] **Step 5: hash.rs — 添加 bump_version()**

受影响的 hash.rs handler：
- handle_hset ✅
- handle_hdel ✅
- handle_hincrby ✅
- handle_hincrbyfloat ✅
- handle_hsetnx ✅

- [ ] **Step 6: zset.rs — 添加 bump_version()**

受影响的 zset.rs handler：
- handle_zadd ✅
- handle_zrem ✅
- handle_zincrby ✅
- handle_zremrangebyrank ✅
- handle_zremrangebyscore ✅

- [ ] **Step 7: stream.rs — 添加 bump_version()**

受影响的 stream.rs handler：
- handle_xadd ✅
- handle_xdel ✅
- handle_xtrim ✅

- [ ] **Step 8: connection.rs — FLUSHDB 加 bump_version()**

在 `handle_flushdb` 中调用 `db::bump_version()`。

- [ ] **Step 9: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 10: Commit**

```bash
git add mini-redis/src/cmd/handlers/*.rs
git commit -m "feat: bump version counter on all write operations"
```

---

### Task 4: 事务测试

**Files:**
- Create: `test-tools/src/tests/transaction.rs`
- Modify: `test-tools/src/tests/mod.rs`
- Modify: `test-tools/src/lib.rs`

- [ ] **Step 1: 创建 test-tools/src/tests/transaction.rs**

```rust
use crate::helpers;
use crate::RedisClient;

pub async fn test_multi_exec_basic(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["SET", "tx:key", "val"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET should be QUEUED");

    let resp = client.cmd(&["GET", "tx:key"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "GET should be QUEUED");

    let resp = client.cmd(&["EXEC"]).await?;
    match &resp {
        mini_redis::resp::RespType::Array(Some(items)) if items.len() == 2 => {
            crate::assert_resp!(items[0].clone(), helpers::simple_str("OK"), "EXEC[0] SET");
            crate::assert_resp!(items[1].clone(), helpers::bulk_str("val"), "EXEC[1] GET");
        }
        _ => return Err(format!("EXEC expected Array(2), got {}", resp)),
    }

    // Verify the key was actually set
    let resp = client.cmd(&["GET", "tx:key"]).await?;
    crate::assert_resp!(resp, helpers::bulk_str("val"), "GET after EXEC");
    Ok(())
}

pub async fn test_multi_discard(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["SET", "tx:discard", "should_not_exist"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET QUEUED");

    let resp = client.cmd(&["DISCARD"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "DISCARD");

    // Key should not exist
    let resp = client.cmd(&["GET", "tx:discard"]).await?;
    crate::assert_resp!(resp, helpers::null_bulk(), "GET after DISCARD should be nil");
    Ok(())
}

pub async fn test_exec_without_multi(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["EXEC"]).await?;
    crate::assert_resp!(resp, helpers::error_str("ERR EXEC without MULTI"), "EXEC without MULTI");
    Ok(())
}

pub async fn test_discard_without_multi(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["DISCARD"]).await?;
    crate::assert_resp!(resp, helpers::error_str("ERR DISCARD without MULTI"), "DISCARD without MULTI");
    Ok(())
}

pub async fn test_nested_multi(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::error_str("ERR MULTI calls can not be nested"), "nested MULTI");

    // Clean up
    let _ = client.cmd(&["DISCARD"]).await?;
    Ok(())
}

pub async fn test_watch_then_exec(client: &mut RedisClient) -> Result<(), String> {
    // Set up a watched key
    let resp = client.cmd(&["SET", "tx:watch", "original"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET watch key");

    let resp = client.cmd(&["WATCH", "tx:watch"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "WATCH");

    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["SET", "tx:watch", "new_val"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET QUEUED");

    let resp = client.cmd(&["EXEC"]).await?;
    // Should succeed because no other connection modified the key
    match &resp {
        mini_redis::resp::RespType::Array(Some(items)) if items.len() == 1 => {
            crate::assert_resp!(items[0].clone(), helpers::simple_str("OK"), "EXEC SET");
        }
        _ => return Err(format!("EXEC expected Array(1), got {}", resp)),
    }
    Ok(())
}

pub async fn test_unwatch(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["WATCH", "tx:unwatch"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "WATCH");

    let resp = client.cmd(&["UNWATCH"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "UNWATCH");
    Ok(())
}
```

注意：`helpers::error_str()` 可能需要定义为 `RespType::Error(...)`。检查 helpers 是否有。如果没有，可以用 `mini_redis::resp::RespType::Error(...)` 直接构造。

- [ ] **Step 2: tests/mod.rs 添加模块**

```rust
pub mod transaction;
```

- [ ] **Step 3: test-tools/src/lib.rs 添加 tree_tests! 条目**

```rust
    ("Transaction", "Transaction") [
        ("MULTI", "New") [
            "MULTI + EXEC basic"    => tests::transaction::test_multi_exec_basic,
            "nested MULTI"          => tests::transaction::test_nested_multi,
        ],
        ("DISCARD", "New") [
            "MULTI + DISCARD"       => tests::transaction::test_multi_discard,
        ],
        ("EXEC", "New") [
            "EXEC without MULTI"    => tests::transaction::test_exec_without_multi,
            "DISCARD without MULTI" => tests::transaction::test_discard_without_multi,
        ],
        ("WATCH", "New") [
            "WATCH then EXEC"       => tests::transaction::test_watch_then_exec,
            "UNWATCH"               => tests::transaction::test_unwatch,
        ],
    ],
```

- [ ] **Step 4: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add test-tools/src/tests/transaction.rs test-tools/src/tests/mod.rs test-tools/src/lib.rs
git commit -m "test: add transaction tests"
```

---

### 验收标准

1. `cargo build --release` 编译通过
2. `cargo run --release --bin test_redis -- Transaction` 所有事务测试通过
3. 已有测试零回归
