---
phase: architecture
issue_id: race-current-db
keywords: [race-condition, global-state, async, tokio, connection-isolation]
files: [mini-redis/src/db.rs, mini-redis/src/main.rs]
severity: high
---

## 现象

多个连接执行 `SELECT N` 后，一个连接可能操作了另一个连接选择的数据库，而非它自己选择的数据库。

## 根本原因

`CURRENT_DB` 是一个 `static AtomicUsize` 全局变量。`dispatch_command()` 在调用 handler 前将 `state.db_index` 写入这个全局变量，handler 再通过 `with_db()` 读取。虽然当前代码在 `set_current_db()` 和 handler 之间没有 `.await` 点，但未来任何人在 dispatch 到 handler 之间添加一个异步操作就会引入 TOCTOU 竞态条件——连接 A 设置 DB=5 后被挂起，连接 B 设置 DB=3，连接 A 恢复后读到 DB=3。

## 修复方案

用 `tokio::task_local!` 替代 `static AtomicUsize`。每个 tokio 任务（每个连接）拥有自己的 DB 索引副本，连接之间完全隔离。`set_current_db()` 改为 `DB_INDEX.with(|cell| cell.set(...))`，`with_db()` 改为 `DB_INDEX.with(|cell| cell.get())`。连接 spawn 时用 `DB_INDEX.scope(Cell::new(0), ...)` 初始化。

## 禁止模式

- 不要在 tokio 任务间共享可变标识符（如数据库索引、请求 ID）——使用 `tokio::task_local!` 而非 `static Atomic*`
- 任何时候在共享状态的读写之间有潜在的 `.await` 点，就需要考虑隔离
