# Phase 04-misc 实现计划

## 涉及文件
- `mini-redis/src/cmd/types.rs`: 添加 Time, Touch { keys: Vec<String> } + name()
- `mini-redis/src/cmd/parsers/admin.rs`: 添加 TIME/TOUCH 解析 + 测试
- `mini-redis/src/cmd/handlers/connection.rs`: 添加 handle_time
- `mini-redis/src/cmd/handlers/key.rs`: 添加 handle_touch
- `mini-redis/src/cmd/dispatch.rs`: 添加 dispatch 臂
- `mini-redis/src/server/registry.rs`: 注册

## 实现细节

### TIME
- 无参数，返回 `[seconds, microseconds]`
- 使用 `SystemTime::now().duration_since(UNIX_EPOCH)` 获取时间
- 返回 `RespType::Array` 包含两个 `RespType::BulkString`

### TOUCH
- TOUCH key [key ...]
- 对每个 key 执行读操作（检查是否存在并更新时间戳）
- 返回成功 touch 的 key 数量（整数）
- 不修改 value 或 expiry，只触发版本更新

## 测试策略
- 解析层单元测试
- TIME: 验证返回格式正确
- TOUCH: 测试 touch 存在的和不存在的 key
