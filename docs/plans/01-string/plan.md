# Phase 01-string 实现计划

## 涉及文件
- `mini-redis/src/cmd/types.rs`: 添加 Setnx/Getex/Getdel 变体 + name() 匹配臂
- `mini-redis/src/cmd/parsers/strs.rs`: 添加 SETNX/GETEX/GETDEL 解析逻辑 + 单元测试
- `mini-redis/src/cmd/handlers/string.rs`: 添加 handle_setnx/handle_getex/handle_getdel
- `mini-redis/src/cmd/dispatch.rs`: 添加 dispatch 匹配臂
- `mini-redis/src/server/registry.rs`: 注册三个命令到 COMMAND INFO

## 实现步骤

### Step 1: types.rs 添加变体
- `Setnx { key: String, value: String }`
- `Getex { key: String, expiry: Option<Duration> }` — EX s | PX ms | EXAT ts | PERSIST
- `Getdel { key: String }`
- name() 返回 "SETNX" / "GETEX" / "GETDEL"

### Step 2: parsers/strs.rs 添加解析
- SETNX: arg[0]=key, arg[1]=value, 必须恰好 2 参数
- GETEX: key + 可选 EX/PX/EXAT/PERSIST 标志
- GETDEL: key 参数
- 添加单元测试

### Step 3: handlers/string.rs 添加 handler
- handle_setnx: 用 with_db 检查键是否存在，不存在时插入返回 1，存在返回 0
- handle_getex: 获取值 + 设置过期（与 SET 的 EX/PX 解析共享模式），PERSIST 清除过期
- handle_getdel: 获取值 + 删除键

### Step 4: dispatch.rs + registry.rs
- 添加匹配臂和注册信息

## 测试策略
- 解析层单元测试（已有测试模块）
- 集成测试涵盖正常路径和边界情况
