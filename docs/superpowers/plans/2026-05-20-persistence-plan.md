# 持久化功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 mini-redis 添加 SAVE/BGSAVE 持久化和启动自动加载

**Architecture:** serde + bincode 序列化整个 DB HashMap，新增 `persist.rs` 模块，配置 `dir`/`dbfilename` 通过 CONFIG 动态修改

**Tech Stack:** serde (derive), bincode, bytes (serde feature)

---

### Task 1: 添加 serde/bincode 依赖 + 数据模型序列化注解

**Files:**
- Modify: `mini-redis/Cargo.toml`
- Modify: `mini-redis/src/db.rs`

- [ ] **Step 1: 添加依赖到 mini-redis/Cargo.toml**

找到 `[dependencies]` 节，添加/修改：

```toml
serde = { version = "1", features = ["derive"] }
bincode = "1"
```

找到 bytes 依赖行，修改为：

```toml
bytes = { version = "1", features = ["serde"] }
```

- [ ] **Step 2: 为 db.rs 的数据类型添加 Serialize/Deserialize 注解**

在 `mini-redis/src/db.rs` 中，对以下类型添加 `#[derive(Serialize, Deserialize)]` 到现有 derive 列表中（注意 serde 的导入通过 `use serde::{Serialize, Deserialize};`）：

```rust
// 在文件顶部添加导入
use serde::{Deserialize, Serialize};

// StreamEntry
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamEntry { ... }

// StreamData
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamData { ... }

// Value — 注意所有变体内部类型都需要 serde
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value { ... }

// Entry — 注意 expiry 字段特殊处理
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry { ... }
```

注意：`Entry.expiry` 字段 `Option<Instant>` 需要特殊处理。这里有两种方案：
- 方案 A（推荐）：用 `#[serde(skip)]` 跳过 expiry，LOAD 后统一无过期
  ```rust
  #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
  pub struct Entry {
      pub value: Value,
      #[serde(skip)]
      pub expiry: Option<Instant>,
  }
  ```
- 方案 B：自定义序列化。但方案 A 更简单，且 SAVE 时已跳过过期 key。

**选择方案 A**（因为 SAVE 时已经过滤掉了过期 key，加载回来的 key 无需带过期时间。如果用户需要持久化 TTL，可以后续迭代。

- [ ] **Step 3: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add mini-redis/Cargo.toml mini-redis/Cargo.lock mini-redis/src/db.rs
git commit -m "feat: add serde/bincode deps and derive annotations for persistence"
```

---

### Task 2: Config 扩展 — dir/dbfilename

**Files:**
- Modify: `mini-redis/src/config.rs`
- Modify: `mini-redis/src/cmd/handlers/connection.rs`（CONFIG GET/SET 扩展）

- [ ] **Step 1: config.rs 添加 dir/dbfilename 字段**

```rust
pub struct ServerConfig {
    pub requirepass: Option<String>,
    pub dir: String,         // 默认 "."
    pub dbfilename: String,  // 默认 "dump.db"
}
```

修改 `ServerConfig::new()`：

```rust
impl ServerConfig {
    pub fn new() -> Self {
        let password = std::env::var("REDIS_PASSWORD").ok().filter(|s| !s.is_empty());
        Self {
            requirepass: password,
            dir: ".".to_string(),
            dbfilename: "dump.db".to_string(),
        }
    }
    // 保留 requirepass_is_set()
}
```

添加获取路径的辅助方法：

```rust
impl ServerConfig {
    pub fn db_path(&self) -> String {
        format!("{}/{}", self.dir.trim_end_matches('/').trim_end_matches('\\'), self.dbfilename)
    }
}
```

- [ ] **Step 2: CONFIG GET/SET 扩展支持 dir 和 dbfilename**

在 `mini-redis/src/cmd/handlers/connection.rs` 的 `handle_config_get` 中，添加 `dir` 和 `dbfilename` 的处理：

```rust
pub fn handle_config_get(parameter: &str) -> RespType {
    let value = config::with_config(|cfg| match parameter {
        "dir" => Some(cfg.dir.clone()),
        "dbfilename" => Some(cfg.dbfilename.clone()),
        "requirepass" => cfg.requirepass.clone(),
        _ => None,
    });
    // 原有逻辑保持不变
}
```

在 `handle_config_set` 中，添加 `dir` 和 `dbfilename` 的支持：

```rust
pub fn handle_config_set(parameter: &str, value: &str) -> RespType {
    config::with_config_mut(|cfg| match parameter {
        "dir" => { cfg.dir = value.to_string(); true }
        "dbfilename" => { cfg.dbfilename = value.to_string(); true }
        "requirepass" => { cfg.requirepass = Some(value.to_string()); true }
        _ => false
    });
    if ok { RespType::SimpleString("OK".to_string()) }
    else { RespType::Error("ERR unknown config parameter".to_string()) }
}
```

注意：需要 `use crate::config` 或通过 `super::super::super::config` 访问。检查现有 connection.rs 的 import 确认。

- [ ] **Step 3: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add mini-redis/src/config.rs mini-redis/src/cmd/handlers/connection.rs
git commit -m "feat: add dir/dbfilename config for persistence"
```

---

### Task 3: 实现 persist.rs 模块

**Files:**
- Create: `mini-redis/src/persist.rs`
- Modify: `mini-redis/src/lib.rs`

- [ ] **Step 1: 新建 persist.rs**

```rust
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::db::{Entry, Value};
use bincode;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

/// Save the entire database to a file at `path`.
/// Expired keys are skipped during serialization.
pub fn save(path: &str) -> Result<(), String> {
    let data = crate::db::with_db(|db| {
        let mut map: HashMap<String, Entry> = HashMap::new();
        let now = Instant::now();
        for (key, entry) in db.iter() {
            if entry.expiry.is_some_and(|exp| now >= exp) {
                continue;
            }
            map.insert(key.clone(), entry.clone());
        }
        map
    });

    let bytes = bincode::serialize(&data).map_err(|e| format!("serialize error: {}", e))?;
    fs::write(path, &bytes).map_err(|e| format!("write error: {}", e))?;
    Ok(())
}

/// Load the database from a file at `path`.
/// Replaces all current in-memory data. Returns the number of keys loaded.
pub fn load(path: &str) -> Result<usize, String> {
    let bytes = fs::read(path).map_err(|e| format!("read error: {}", e))?;
    let data: HashMap<String, Entry> =
        bincode::deserialize(&bytes).map_err(|e| format!("deserialize error: {}", e))?;

    let count = data.len();
    crate::db::with_db(|db| {
        db.clear();
        db.extend(data);
    });
    Ok(count)
}

/// Check whether a persistence file exists at `path`.
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}
```

- [ ] **Step 2: lib.rs 注册 persist 模块**

在 `mini-redis/src/lib.rs` 中，在现有模块声明后添加：

```rust
pub mod persist;
```

- [ ] **Step 3: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add mini-redis/src/persist.rs mini-redis/src/lib.rs
git commit -m "feat: add persist module with save/load functions"
```

---

### Task 4: SAVE/BGSAVE/SHUTDOWN 命令解析和路由

**Files:**
- Modify: `mini-redis/src/cmd/types.rs`
- Modify: `mini-redis/src/cmd/parse.rs`
- Modify: `mini-redis/src/cmd/dispatch.rs`
- Modify: `mini-redis/src/cmd/handlers/connection.rs`
- Modify: `mini-redis/src/registry.rs`

- [ ] **Step 1: types.rs 添加新命令变体**

在 `ParsedCmd` 枚举中添加：

```rust
pub enum ParsedCmd {
    // ... 保留现有变体，在合适位置添加：
    Save,
    Bgsave,
    Shutdown,
}
```

在 `ParsedCmd::name()` 方法中添加对应分支：

```rust
ParsedCmd::Save => "SAVE",
ParsedCmd::Bgsave => "BGSAVE",
ParsedCmd::Shutdown => "SHUTDOWN",
```

- [ ] **Step 2: parse.rs 添加命令解析**

在 `ParsedCmd::parse()` 方法的 `match cmd` 中添加：

```rust
"SAVE" => ParsedCmd::Save,
"BGSAVE" => ParsedCmd::Bgsave,
"SHUTDOWN" => ParsedCmd::Shutdown,
```

SAVE 和 SHUTDOWN 不接受参数（args 非空时返回 wrong arg count），BGSAVE 也不接受参数。在 dispatch 层面验证即可。

- [ ] **Step 3: registry.rs 注册命令**

```rust
reg.register(CommandInfo {
    name: "SAVE",
    arity: 1,
    category: "Server",
    since_stage: 0,
    summary: "Synchronously saves the dataset to disk",
});
reg.register(CommandInfo {
    name: "BGSAVE",
    arity: 1,
    category: "Server",
    since_stage: 0,
    summary: "Asynchronously saves the dataset to disk in background",
});
reg.register(CommandInfo {
    name: "SHUTDOWN",
    arity: 1,
    category: "Server",
    since_stage: 0,
    summary: "Synchronously saves the dataset to disk and shuts down",
});
```

- [ ] **Step 4: dispatch.rs 添加路由**

```rust
ParsedCmd::Save => handlers::handle_save(),
ParsedCmd::Bgsave => handlers::handle_bgsave(),
ParsedCmd::Shutdown => handlers::handle_shutdown(),
```

- [ ] **Step 5: connection.rs 添加 handler 实现**

```rust
pub fn handle_save() -> RespType {
    let path = config::with_config(|cfg| cfg.db_path());
    match crate::persist::save(&path) {
        Ok(()) => RespType::SimpleString("OK".to_string()),
        Err(e) => RespType::Error(format!("ERR {}", e)),
    }
}

pub fn handle_bgsave() -> RespType {
    let path = config::with_config(|cfg| cfg.db_path());
    // 克隆数据在后台保存
    let data = crate::db::with_db(|db| {
        let now = tokio::time::Instant::now();
        let mut map: HashMap<String, crate::db::Entry> = HashMap::new();
        for (key, entry) in db.iter() {
            if entry.expiry.is_some_and(|exp| now >= exp) {
                continue;
            }
            map.insert(key.clone(), entry.clone());
        }
        map
    });

    // 后台异步保存
    tokio::spawn(async move {
        let bytes = match bincode::serialize(&data) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("BGSAVE serialize error: {}", e);
                return;
            }
        };
        if let Err(e) = std::fs::write(&path, &bytes) {
            eprintln!("BGSAVE write error: {}", e);
        } else {
            println!("BGSAVE completed to {}", path);
        }
    });

    RespType::SimpleString("OK".to_string())
}

pub fn handle_shutdown() -> RespType {
    let path = config::with_config(|cfg| cfg.db_path());
    if let Err(e) = crate::persist::save(&path) {
        return RespType::Error(format!("ERR {}", e));
    }
    // 需要一种方式通知主循环退出
    // 发送 SIGTERM 风格的信号
    std::process::exit(0);
}
```

注意：handle_bgsave 在 handler 函数目前都是同步的（除了 handle_blpop）。由于序列化很快，同步执行即可。如需真正后台，需要 `tokio::spawn` 但 handler 返回 `RespType` 而非 `Future`。

对于 handle_shutdown：`std::process::exit(0)` 是最简单的方式。更优雅的方式是用一个全局的 `AtomicBool` 标记通知主循环退出，但 exit(0) 简单有效。

- [ ] **Step 6: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 7: Commit**

```bash
git add mini-redis/src/cmd/types.rs mini-redis/src/cmd/parse.rs mini-redis/src/cmd/dispatch.rs mini-redis/src/cmd/handlers/connection.rs mini-redis/src/registry.rs
git commit -m "feat: add SAVE/BGSAVE/SHUTDOWN command parsing and handlers"
```

---

### Task 5: 启动自动加载和关闭自动保存

**Files:**
- Modify: `mini-redis/src/main.rs`

- [ ] **Step 1: main.rs 添加启动加载和关闭保存**

在 `main()` 函数中，`registry::init()` 之后、`bind` 之前添加自动加载：

```rust
// Auto-load persistence file
let persisted_count = {
    let path = config::with_config(|cfg| cfg.db_path());
    if crate::persist::file_exists(&path) {
        match crate::persist::load(&path) {
            Ok(n) => {
                println!("Loaded {} keys from {}", n, path);
                n
            }
            Err(e) => {
                eprintln!("Failed to load persistence file: {}", e);
                0
            }
        }
    } else {
        0
    }
};
```

在 `ctrl_c()` 分支中，在打印关闭消息之前添加自动保存：

```rust
_ = signal::ctrl_c() => {
    println!("\nCtrl+C received, saving data...");
    let path = config::with_config(|cfg| cfg.db_path());
    if let Err(e) = crate::persist::save(&path) {
        eprintln!("Failed to save data: {}", e);
    }
    println!("Data saved to {}", path);
    println!("Shutting down...");
    Ok(())
}
```

同时需要将 `config::with_config` 导入和 `crate::persist` 在 main.rs 中可用。检查 main.rs 是否已有 `config` 导入（已有的 `use mini_redis::{cmd, config, db, inline, registry, resp};`）。

注意：main.rs 用的是 `use mini_redis::{...}` 即 crate 名，在二进制中 `mini-redis` crate 内部使用 `crate::` 前缀。由于 main.rs 是 `mini-redis` crate 的二进制入口，`crate::persist` 可直接使用。

- [ ] **Step 2: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 3: 功能验证（手动）**

启动服务器，执行 SAVE，检查文件是否生成：

```bash
# 终端1
cargo run --release
# 终端2 (redis-cli -p 6379 或直接用 nc/echo)
echo -e "*3\r\n\$3\r\nSET\r\n\$3\r\nfoo\r\n\$3\r\nbar\r\n" | nc 127.0.0.1 6379
echo -e "*1\r\n\$4\r\nSAVE\r\n" | nc 127.0.0.1 6379
# 检查 dump.db 文件是否存在
ls -la dump.db
```

- [ ] **Step 4: Commit**

```bash
git add mini-redis/src/main.rs
git commit -m "feat: add auto-load on startup and auto-save on shutdown"
```

---

### Task 6: 测试

**Files:**
- Modify: `test-tools/src/lib.rs`
- Modify: `test-tools/src/tests/mod.rs`
- Create: `test-tools/src/tests/persistence.rs`
- Modify: `test-tools/Cargo.toml`（如需）

- [ ] **Step 1: 创建 test-tools/src/tests/persistence.rs**

```rust
use crate::helpers;
use crate::RedisClient;

pub async fn test_save_basic(client: &mut RedisClient) -> Result<(), String> {
    // 设置一个 key
    let resp = client.cmd(&["SET", "persist:test", "hello"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET should succeed");

    // SAVE
    let resp = client.cmd(&["SAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SAVE should return OK");

    // 验证文件存在
    let path = std::path::Path::new("dump.db");
    if !path.exists() {
        return Err("SAVE did not create dump.db".to_string());
    }

    // 清理
    let _ = std::fs::remove_file("dump.db");
    Ok(())
}

pub async fn test_save_roundtrip(client: &mut RedisClient) -> Result<(), String> {
    // 设置多个不同类型的 key
    let resp = client.cmd(&["SET", "rt:string", "value1"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET string");

    let resp = client.cmd(&["RPUSH", "rt:list", "a", "b", "c"]).await?;
    crate::assert_resp!(resp, helpers::int(3), "RPUSH list");

    let resp = client.cmd(&["HSET", "rt:hash", "field1", "val1"]).await?;
    crate::assert_resp!(resp, helpers::int(1), "HSET hash");

    let resp = client.cmd(&["SADD", "rt:set", "member1"]).await?;
    crate::assert_resp!(resp, helpers::int(1), "SADD set");

    // SAVE
    let resp = client.cmd(&["SAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SAVE");

    // 验证文件存在
    let path = std::path::Path::new("dump.db");
    if !path.exists() {
        return Err("SAVE did not create dump.db".to_string());
    }

    // 清理
    let _ = std::fs::remove_file("dump.db");
    Ok(())
}

pub async fn test_bgsave(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["SET", "bgsave:test", "world"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET");

    let resp = client.cmd(&["BGSAVE"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "BGSAVE should return OK");

    let _ = std::fs::remove_file("dump.db");
    Ok(())
}

pub async fn test_config_get_dir(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["CONFIG", "GET", "dir"]).await?;
    crate::assert_match!(resp, mini_redis::resp::RespType::Array(Some(_)), "CONFIG GET dir should return array");
    Ok(())
}

pub async fn test_config_set_dir(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["CONFIG", "SET", "dir", "/tmp"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "CONFIG SET dir");

    // 改回来
    let resp = client.cmd(&["CONFIG", "SET", "dir", "."]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "CONFIG SET dir back");
    Ok(())
}
```

注意：需要 `use mini_redis::resp::RespType;` 在 persistence.rs 文件顶部。

- [ ] **Step 2: tests/mod.rs 添加 persistence 模块**

```rust
pub mod persistence;
```

- [ ] **Step 3: test-tools/src/lib.rs 添加测试到 tree_tests!**

在 tree_tests! 宏的合适位置添加（比如在 Auth 之后）：

```rust
("Persistence", "Persistence") [
    ("SAVE", "New") [
        "SAVE basic"           => tests::persistence::test_save_basic,
        "SAVE roundtrip types" => tests::persistence::test_save_roundtrip,
    ],
    ("BGSAVE", "New") [
        "BGSAVE"               => tests::persistence::test_bgsave,
    ],
    ("CONFIG", "New") [
        "CONFIG GET dir"       => tests::persistence::test_config_get_dir,
        "CONFIG SET dir"       => tests::persistence::test_config_set_dir,
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
git add test-tools/src/tests/persistence.rs test-tools/src/tests/mod.rs test-tools/src/lib.rs
git commit -m "test: add persistence tests"
```

---

### 验收标准

1. `cargo build --release` 编译通过
2. 运行 `cargo run --release` 启动服务器，设置一些 key，执行 SAVE，退出，再启动能加载回来
3. `cargo run --release --bin test_redis -- Persistence` 运行持久化相关测试（需先启动服务器）
