---
phase: architecture
issue_id: mutex-poisoning-cascade
keywords: [panic, mutex-poison, error-isolation, hyperloglog]
files: [mini-redis/src/db.rs]
severity: high
---

## 现象

hyperloglog 命令导致服务器内部 panic（`sip1.finish() == sip2.finish()`），随后 panic 毒化了全局 `DBS` mutex，导致**所有后续数据库操作**都以 `PoisonError` 连锁 panic 告终。

## 根本原因

`DBS.lock().unwrap()` 在 Mutex 被毒化后 panic。Rust 的 `std::sync::Mutex` 在一个线程 panic 时被毒化（poisoned），后续线程 `lock()` 返回 `Err(PoisonError)`。`unwrap()` 导致该线程也 panic，形成连锁反应。

触发源是 hyperloglog crate v1.0.3 的内部 SipHash 实现 bug（双 SipHash 实例结果不一致的断言失败），本应在 hyperloglog handler 处隔离的错误通过 mutex poisoning 污染了整个系统。

## 修复方案

所有 `DBS.lock().unwrap()` 替换为：

```rust
fn lock_dbs() -> MutexGuard<'static, Vec<HashMap<String, Entry>>> {
    DBS.lock().unwrap_or_else(|e| e.into_inner())
}
```

`into_inner()` 返回 Mutex 内部的数据，忽略中毒状态。由于 Rust 的 Mutex 只保护数据完整性而非一致性，panic 后数据内部可能不一致，但比整个服务器崩溃要好——客户端得到错误响应而非断连。

## 后续

hyperloglog 已全部删除。如需恢复该功能，应替换为稳定的 crate 或在 handler 层用 `std::panic::catch_unwind` 隔离。

## 禁止模式

- 在服务器/长期运行进程中，`Mutex::lock().unwrap()` 必须考虑中毒恢复，或至少用 `catch_unwind` 隔离可能 panic 的外部代码
- 全局 Mutex 是单点故障——一个 handler 的 panic 可以瘫痪整个系统
