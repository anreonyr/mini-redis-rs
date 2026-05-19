# PRD: Redis 缺失命令补全

## 背景
当前 Redis 项目已支持 36 个命令和 6 种数据类型（String、List、Stream、Hash、Set、ZSet），但缺少基础 key 管理、过期管理、String 增值操作、以及各数据类型的常用命令。需要补全约 34 个命令，使服务器具备实用级别的 Redis 兼容性。

## 目标
按优先级分批补全以下命令，全部通过测试。

## 功能列表

### Phase 01: 基础命令
1. **DEL** key [key ...] — 删除一个或多个 key，返回实际删除数量
2. **EXISTS** key [key ...] — 检查 key 是否存在，返回存在数量
3. **TYPE** key — 返回 key 的数据类型名称（string/list/stream/hash/set/zset/none）
4. **KEYS** pattern — 匹配并返回 key 名称（支持 * 通配符）
5. **DBSIZE** — 返回数据库中 key 总数
6. **EXPIRE** key seconds — 设置 key 过期时间（秒），返回 1（成功）或 0（key 不存在）
7. **TTL** key — 返回剩余过期秒数（-2=已过期/不存在，-1=无过期，>=0=剩余秒数）
8. **PERSIST** key — 移除 key 的过期时间，返回 1（成功）或 0

### Phase 02: 数据类型核心命令
9. **INCR / DECR** key — 值自增/自减 1，key 不存在时初始化为 0
10. **INCRBY / DECRBY** key n — 按指定值自增/自减
11. **APPEND** key value — 追加字符串值，返回新长度
12. **STRLEN** key — 返回字符串字节长度
13. **MGET** key [key ...] — 批量获取值
14. **MSET** key value [key value ...] — 批量设置值
15. **RPOP** key [count] — 从列表右侧弹出
16. **LINDEX** key index — 按索引获取元素
17. **LREM** key count value — 按值移除元素
18. **LTRIM** key start stop — 截取列表范围
19. **SPOP** key [count] — 随机弹出集合成员
20. **SRANDMEMBER** key [count] — 随机获取集合成员
21. **SUNION** key [key ...] — 集合并集
22. **SINTER** key [key ...] — 集合交集
23. **SDIFF** key [key ...] — 集合差集

### Phase 03: 高级命令
24. **ZREM** key member [member ...] — 删除成员
25. **ZCARD** key — 返回成员数量
26. **ZCOUNT** key min max — 按分数范围计数
27. **ZRANGEBYSCORE** key min max [WITHSCORES] [LIMIT offset count] — 按分数范围查询
28. **ZINCRBY** key incr member — 增加成员分数
29. **ZREVRANGE** key start stop [WITHSCORES] — 反向顺序范围查询
30. **ZREVRANK** key member — 获取成员反向排名
31. **INFO** [section] — 返回服务器信息
32. **CONFIG GET** parameter — 获取配置项

## 非功能需求
- 与 Redis 官方协议兼容
- 与现有项目代码风格一致（5 文件模式）
- WRONGTYPE 错误处理
- 过期 key 正确处理（所有读操作检查过期）

## 技术栈
Rust, Tokio, RESP 协议

## 验收标准
1. `cargo build --release` 编译通过
2. `cargo run --release --bin test_redis` 所有测试通过
3. 已有 56 个测试无回归
