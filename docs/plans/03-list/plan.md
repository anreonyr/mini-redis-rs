# Phase 03-list 实现计划

## 涉及文件
- `mini-redis/src/cmd/types.rs`: 添加 Brpop { keys: Vec<String>, timeout: u64 }
- `mini-redis/src/cmd/parsers/lists.rs`: BRPOP 解析（与 BLPOP 相同逻辑）
- `mini-redis/src/cmd/handlers/list.rs`: try_brpop + handle_brpop
- `mini-redis/src/cmd/dispatch.rs`: 添加 dispatch 臂
- `mini-redis/src/server/registry.rs`: 注册 BRPOP

## 实现细节
- BRPOP 完全对称于 BLPOP，唯一区别是 `list.pop_back()` 而不是 `list.pop_front()`
- 可以在 list.rs 中泛化或直接复制 BLPOP 模式的第二个版本
- 阻塞机制复用 Weak<Notify> + BlpopGuard（已存在）
- waiters.rs 已支持多键注册

## 测试策略
- 解析层测试
- 集成测试：basic pop、timeout、multi-key
