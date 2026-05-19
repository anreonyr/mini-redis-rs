---
phase: architecture
issue_id: process-exit-bypasses-cleanup
keywords: [shutdown, graceful, resource-leak, connection-drain]
files: [mini-redis/src/cmd/handlers/connection.rs, mini-redis/src/main.rs, mini-redis/src/shutdown.rs]
severity: medium
---

## 现象

`SHUTDOWN` 命令直接调用 `std::process::exit(0)`，跳过了：
- 活跃连接的优雅关闭（它们被操作系统强行终止）
- 后台持久化任务的完成等待
- eviction 任务的停止

## 根本原因

handler 运行在 tokio 任务中，无法直接停止 tokio runtime 或等待其他任务完成。`exit(0)` 是最简单的"让它停"的做法，但也是破坏性最强的。

## 修复方案

新增 `shutdown.rs` 模块，使用 `AtomicBool` 作为关机信号：

1. **`handle_shutdown()`** — 先保存数据（`persist::save().await`），再设置全局 shutdown flag，返回 OK
2. **`dispatch.rs`** — SHUTDOWN 处理后设置 `state.quit = true`，连接循环退出，连接任务结束
3. **accept loop** — 每次 accept 前检查 `shutdown::is_requested()`，收到信号后停止接受新连接
4. **eviction 任务** — 循环中检查 shutdown 信号，主动退出
5. **主线 `select!`** — accept loop 返回后（或 Ctrl+C 后）执行持久化保存，然后 `join_next()` 等待所有已连接的连接自然完成
6. **Ctrl+C** — 先发 shutdown 信号让 accept loop 停止，然后保存数据，等待连接 drain

关闭流程：
```
SHUTDOWN → save data → accept loop 停 → state.quit → 连接断开 → JoinSet drain → 进程退出
Ctrl+C   → shutdown flag → accept loop 停 → save data → JoinSet drain → 进程退出
```

## 禁止模式

- 在任何网络服务器中，`process::exit()` 应作为最后手段：它不运行析构函数、不等待异步任务、不发送 FIN 包
- 一个命令 handler 不应直接终止进程——它应通过信号/通道通知协调者
