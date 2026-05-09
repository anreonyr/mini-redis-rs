# PRD: Redis HASH 命令实现

## 背景
当前 Redis 项目已支持 String、List、Stream 等数据类型，HASH 类型尚未实现。需要添加 HSET、HGET、HGETALL 三个基础命令。

## 目标
实现 HASH 类型的基础命令，通过现有的 HASH 测试用例。

## 功能列表
1. HSET — 设置 hash 字段值，返回 1（新建）或 0（覆盖）
2. HGET — 获取 hash 字段值，字段不存在返回 nil
3. HGETALL — 获取所有 field-value 对

## 非功能需求
- 与 Redis 官方协议兼容
- 与现有项目代码风格一致
- WRONGTYPE 错误处理

## 技术栈
Rust, Tokio, RESP 协议

## 验收标准
1. `HSET myhash field1 value1` → `:1`
2. `HSET myhash field1 value2` → `:0`
3. `HGET myhash field1` → `$6\r\nvalue2`
4. `HGET myhash nonexist` → `$-1`
5. `HGETALL myhash` → `*4\r\n$6\r\nfield1\r\n...`
