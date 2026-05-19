# 事务功能设计文档

## 概述
为 mini-redis 添加完整的事务支持：MULTI、EXEC、DISCARD、WATCH、UNWATCH。

## 架构

每个连接维护一个 `ConnectionState`，新增 `transaction: Option<TransactionState>` 字段：

```rust
pub struct TransactionState {
    queue: Vec<ParsedCmd>,        // 事务队列
    watching: HashMap<String, u64>, // key → 记录的版本号
}
```

## 版本号乐观锁

`db.rs` 中：
- 全局 `VERSION_COUNTER: AtomicU64`
- `Entry` 新增 `version: u64` 字段
- `bump_version() -> u64` — 递增并返回新版本号
- `key_version(key) -> Option<u64>` — 读取 key 的当前版本

每次写操作创建或修改 key 时调用 `bump_version()` 并更新 `entry.version`。

受影响的 handler（全部写操作）：SET、GETSET、SETRANGE、APPEND、MSET、MSETNX、DEL、RENAME、RENAMENX、RPUSH、LPUSH、RPOP、LPOP、LREM、LTRIM、LSET、RPOPLPUSH、SADD、SREM、SPOP、SMOVE、HSET、HDEL、HINCRBY、HINCRBYFLOAT、HSETNX、ZADD、ZREM、ZINCRBY、ZREMRANGEBYRANK、ZREMRANGEBYSCORE、XADD、XDEL、XTRIM、FLUSHDB

## 命令行为

### MULTI
- 如已在事务中 → 返回 `-ERR MULTI calls can not be nested`
- 否则创建空的 `TransactionState`，返回 `+OK`

### EXEC
- 如不在事务中 → 返回 `-ERR EXEC without MULTI`
- 检查所有 `watching` key 的版本：
  - 任一变化 → 清空事务状态，返回 `*-nil`（空数组/nil）
- 否则顺序执行 `queue`，收集结果数组返回
- 清空事务状态

### DISCARD
- 如不在事务中 → 返回 `-ERR DISCARD without MULTI`
- 清空 queue + watching，退出事务模式，返回 `+OK`

### WATCH key [key ...]
- 记录每个 key 当前版本到 `watching`
- 返回 `+OK`
- 在事务中也可执行（直接执行，不入队）

### UNWATCH
- 清空 `watching`，返回 `+OK`
- 在事务中也可执行

### 事务内的命令入队规则
- WATCH/UNWATCH/MULTI/EXEC/DISCARD 在事务内直接执行
- 其他命令入队到 `queue`，返回 `+QUEUED`
- 入队时不执行，不检查参数有效性（Redis 行为：部分检查在入队时做，简化为 EXEC 时检查）

## 文件变更

| 文件 | 变更 |
|------|------|
| `mini-redis/src/db.rs` | Entry 加 version、bump_version()、key_version() |
| `mini-redis/src/cmd/types.rs` | ParsedCmd 加 Multi/Exec/Discard/Watch/Unwatch |
| `mini-redis/src/cmd/parse.rs` | 解析 5 个新命令 |
| `mini-redis/src/cmd/dispatch.rs` | 路由到 handler |
| `mini-redis/src/cmd/handlers/connection.rs` | 实现 5 个 handler |
| `mini-redis/src/cmd/handlers/*.rs` | 所有写 handler 调用 bump_version() |
| `mini-redis/src/cfg/auth.rs` | ConnectionState 加 transaction 字段 |
| `mini-redis/src/registry.rs` | 注册 5 个命令 |
| `test-tools/src/tests/transaction.rs` | 事务测试 |
| `test-tools/src/tests/mod.rs` | 加模块声明 |
| `test-tools/src/lib.rs` | tree_tests! 加测试条目 |

## 测试

- 测试基本事务流程（MULTI + 入队 + EXEC）
- 测试 DISCARD 放弃事务
- 测试 WATCH 后其他连接修改 key，EXEC 返回 nil
- 测试 WATCH 后无修改，EXEC 正常执行
- 测试嵌套 MULTI 报错
- 测试 UNWATCH
- 测试不在事务中调用 EXEC/DISCARD 报错
