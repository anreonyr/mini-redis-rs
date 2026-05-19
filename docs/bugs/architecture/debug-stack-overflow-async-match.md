---
phase: architecture
issue_id: debug-stack-overflow-async-match
keywords: [stack-overflow, debug-mode, async, macro, enum-layout]
files: [test-tools/src/lib.rs]
severity: medium
---

## 现象

TUI 测试运行器（`tui_redis.exe`）启动后 stack overflow，退出码 `0xc00000fd`。只在 debug 模式下复现。

## 根本原因

`tree_tests!` 宏生成的 `run_test()` 函数包含一个 150+ 分支的 `async move { match (subcat, name) { ... } }`。在 debug 模式下，Rust 编译器生成的 async state machine 枚举为每个 `.await` 变体分配完整的内联空间，导致枚举体积巨大（远超 4MB worker 栈）。

`Box::pin` 虽然将最终 future 放在堆上，但 `async move { match ... }` 块的初始化仍在栈上进行——枚举的创建需要临时内存。

## 修复方案

按测试类别分派：`run_test` 不再是 async 函数，而是一个同步 match，每个 match arm 创建**仅包含该类别的测试 handler**的小型 async block：

```rust
pub fn run_test(def, client) -> Pin<Box<dyn Future>> {
    match def.category {
        "Base" => Box::pin(async move {
            // 仅 ~10 个 match arm
        }),
        "Key" => Box::pin(async move { /* ~15 个 */ }),
        // ... 共 15 个类别
    }
}
```

每个 async block 的枚举只有 ~5-20 个变体，栈上创建安全。

## 禁止模式

- debug 模式下，一个 async block 内包含 50+ 个 `.await` 分支的 match 可能撑爆栈
- `Box::pin` 只保证最终 future 在堆上，不保证初始化过程在栈上安全
