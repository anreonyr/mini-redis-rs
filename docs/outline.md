# OUTLINE: Redis 缺失命令补全

## Phase 01-scaffold: 基础命令

### 任务 01-key-mgmt: DEL, EXISTS, TYPE, KEYS, DBSIZE <!-- @depends:(none) @code_agents:2 -->
实现 5 个 key 管理命令的解析、路由、处理逻辑和测试。
- 新增文件：`cmd/handlers/key.rs`、`test-tools/src/tests/key.rs`
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/mod.rs`、`test-tools/src/tests/mod.rs`、`test-tools/src/lib.rs`
- 验收：所有 key 管理测试通过 + 编译无误

### 任务 01-expiry: EXPIRE, TTL, PERSIST <!-- @depends:01-key-mgmt @code_agents:2 -->
实现 3 个过期管理命令。
- 新增文件：`cmd/handlers/expiry.rs`
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/mod.rs`、`test-tools/src/tests/expiry.rs`、`test-tools/src/lib.rs`
- 验收：所有过期管理测试通过 + 编译无误

## Phase 02-core: 数据类型核心命令

### 任务 02-string: INCR, DECR, INCRBY, DECRBY, APPEND, STRLEN, MGET, MSET <!-- @depends:01-expiry @code_agents:2 -->
实现 8 个字符串增值和批量操作命令。
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/string.rs`、`test-tools/src/tests/string.rs`、`test-tools/src/lib.rs`
- 验收：所有 string 扩展测试通过 + 编译无误

### 任务 02-list: RPOP, LINDEX, LREM, LTRIM <!-- @depends:02-string @code_agents:2 -->
实现 4 个列表操作命令。
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/list.rs`、`test-tools/src/tests/list.rs`、`test-tools/src/lib.rs`
- 验收：所有 list 扩展测试通过 + 编译无误

### 任务 02-set: SPOP, SRANDMEMBER, SUNION, SINTER, SDIFF <!-- @depends:02-list @code_agents:2 -->
实现 5 个集合操作命令。
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/set.rs`、`test-tools/src/tests/set.rs`、`test-tools/src/lib.rs`
- 验收：所有 set 扩展测试通过 + 编译无误

## Phase 03-polish: 收尾命令

### 任务 03-zset: ZREM, ZCARD, ZCOUNT, ZRANGEBYSCORE, ZINCRBY, ZREVRANGE, ZREVRANK <!-- @depends:02-set @code_agents:2 -->
实现 7 个有序集合高级命令。
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/zset.rs`、`test-tools/src/tests/zset.rs`、`test-tools/src/lib.rs`
- 验收：所有 zset 扩展测试通过 + 编译无误

### 任务 03-server: INFO, CONFIG GET <!-- @depends:03-zset @code_agents:1 -->
实现 2 个服务器管理命令。
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`registry.rs`、`cmd/handlers/connection.rs`、`test-tools/src/tests/server.rs`、`test-tools/src/lib.rs`
- 验收：所有 server 测试通过 + 编译无误 + 最终回归验证

## Phase 04-auth: 鉴权系统

### 任务 04-auth: AUTH + requirepass <!-- @depends:(none) @code_agents:1 -->
实现 Redis 密码鉴权系统。
- 新增文件：`config.rs`、`cmd/auth.rs`、`test-tools/src/tests/auth.rs`
- 修改文件：`cmd/types.rs`、`cmd/parse.rs`、`cmd/dispatch.rs`、`cmd/handlers/connection.rs`、`cmd/mod.rs`、`cmd/handlers/mod.rs`、`lib.rs`、`registry.rs`、`main.rs`、`test-tools/src/tests/mod.rs`、`test-tools/src/lib.rs`
- 验收：所有 auth 测试通过 + 已有测试零回归
