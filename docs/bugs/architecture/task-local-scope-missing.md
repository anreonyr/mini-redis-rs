---
phase: architecture
issue_id: task-local-scope-missing
keywords: [panic, task-local, background-task, eviction, bgsave, ctrl-c]
files: [mini-redis/src/main.rs, mini-redis/src/cmd/handlers/connection.rs]
severity: critical
---

## 现象

服务器启动后立即 panic：`cannot access a task-local storage value without setting it first`，panic 位置在 `db.rs:110` 的 `DB_INDEX.with(|cell| cell.get())`。

## 根本原因

`with_db()` 现在依赖 `tokio::task_local!` 的 `DB_INDEX`，该值必须通过 `DB_INDEX.scope(...)` 设置后才可访问。以下三个后台任务/代码路径没有建立 scope：

1. **eviction 任务**（`main.rs`）：`tokio::spawn` 直接调用 `db::with_db()`
2. **Ctrl+C 处理**（`main.rs`）：`signal::ctrl_c()` 分支内调用 `persist::save()` → `with_db()`
3. **BGSAVE 后台任务**（`connection.rs`）：`tokio::spawn` 内调用 `persist::save()` → `with_db()`

## 修复方案

在每个后台任务和独立 async 块的入口处包一层 `DB_INDEX.scope(Cell::new(0), ...)`：

- eviction 任务：`tokio::spawn(async { DB_INDEX.scope(Cell::new(0), async { loop { ... } }).await; });`
- Ctrl+C 分支：`DB_INDEX.scope(Cell::new(0), async { persist::save(&path).await }).await;`
- BGSAVE spawn：`tokio::spawn(async move { DB_INDEX.scope(Cell::new(0), async { persist::save(&path).await }).await; });`

## 禁止模式

- 引入 `tokio::task_local!` 后，**所有**调用了 `.get()` 或 `.with()` 的代码路径都必须有 `scope()` 包裹
- 特别要注意后台 `spawn`、`spawn_blocking`、signal handler、定时器回调等不继承调用方 task-local scope
