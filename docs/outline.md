# OUTLINE: Redis HASH 命令

## 阶段 01-test-hash

### 任务 1.1: HSET/HGET 解析与存储 <!-- @depends:(none) @code_agents:1 -->
实现 HSET 和 HGET 命令的 RESP 解析、Value::Hash 存储和读取。
涉及文件：cmd/parse.rs, cmd/types.rs, db.rs, cmd/handlers/hash.rs

### 任务 1.2: HGETALL 实现 <!-- @depends:1.1 -->
实现 HGETALL 命令，遍历 Hash 所有 field-value 对。
涉及文件：cmd/handlers/hash.rs

### 任务 1.3: HDEL 实现 <!-- @depends:1.2 @code_agents:1 -->
实现 HDEL 命令，从 Hash 中删除指定字段，返回实际删除数量。
涉及文件：cmd/handlers/hash.rs
