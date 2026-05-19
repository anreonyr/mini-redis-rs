# Phase 02-expiry 实现计划

## 涉及文件
- `mini-redis/src/cmd/types.rs`: 添加 Pexpi.../Pttl/Pexipreat/Expipreat/Expiretime/Pexpipetime + name()
- `mini-redis/src/cmd/parsers/admin.rs`: 添加 PEXPIRE/PTTL/PEXPIREAT/EXPIREAT/EXPIRETIME/PEXPIRETIME 解析 + 测试
- `mini-redis/src/cmd/handlers/expiry.rs`: 添加对应的 handler 函数
- `mini-redis/src/cmd/dispatch.rs`: 添加 dispatch 臂
- `mini-redis/src/server/registry.rs`: 注册

## 实现步骤

### Step 1: types.rs
- `Pexpire { key: String, milliseconds: u64 }`
- `Pttl { key: String }`
- `Pexpireat { key: String, timestamp_ms: u64 }`
- `Expireat { key: String, timestamp: u64 }`
- `Expiretime { key: String }`
- `Pexpiretime { key: String }`

### Step 2: parsers/admin.rs
- PEXPIRE: key + ms, 2 参数
- PTTL: key, 1 参数
- PEXPIREAT: key + ms-timestamp, 2 参数
- EXPIREAT: key + unix-ts-seconds, 2 参数
- EXPIRETIME: key, 1 参数
- PEXPIRETIME: key, 1 参数
- 添加测试

### Step 3: handlers/expiry.rs
- handle_pexpire: 类似 handle_expire，但用 Duration::from_millis
- handle_pttl: 类似 handle_ttl，返回值 * 1000（毫秒）
- handle_pexpireat: 从毫秒时间戳转换为 Duration
- handle_expireat: 从秒时间戳转换为 Duration
- handle_expiretime: 从 Instant expiry 计算 Unix 秒时间戳
- handle_pexpiretime: 从 Instant expiry 计算 Unix 毫秒时间戳

### Step 4: dispatch.rs + registry.rs

## 测试策略
- 解析层单元测试
- 需要集成测试验证毫秒精度和时间戳过期
