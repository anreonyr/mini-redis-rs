# OUTLINE: P1 级 Redis 命令实现

## 阶段 01-list-move: 列表移动操作

### 任务 1.1: BRPOPLPUSH 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
BRPOPLPUSH source destination timeout 的解析与 handler。阻塞式从 source 右端弹出并推入 destination 左端。涉及文件：types.rs, parsers/lists.rs, handlers/list.rs, dispatch.rs, registry.rs

### 任务 1.2: LMOVE 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
LMOVE source destination LEFT|RIGHT LEFT|RIGHT 解析与 handler。原子列表移动，指定左右方向。涉及文件：types.rs, parsers/lists.rs, handlers/list.rs, dispatch.rs, registry.rs

### 任务 1.3: BLMOVE 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
BLMOVE source destination LEFT|RIGHT LEFT|RIGHT timeout 解析与 handler。阻塞式 LMOVE。涉及文件：types.rs, parsers/lists.rs, handlers/list.rs, dispatch.rs, registry.rs

## 阶段 02-zset-pop: 有序集合弹出操作

### 任务 2.1: ZPOPMIN / ZPOPMAX 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
ZPOPMIN key [count] 和 ZPOPMAX key [count] 的解析与 handler。从有序集合弹出最低/最高分元素。涉及文件：types.rs, parsers/category, handlers/zset.rs, dispatch.rs, registry.rs

### 任务 2.2: BZPOPMIN / BZPOPMAX 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
BZPOPMIN key [key ...] timeout 和 BZPOPMAX key [key ...] timeout 的解析与 handler。阻塞式弹出。涉及文件：types.rs, parsers, handlers/zset.rs, dispatch.rs, registry.rs

## 阶段 03-set-store: 集合存储操作

### 任务 3.1: SUNIONSTORE / SINTERSTORE / SDIFFSTORE 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
SUNIONSTORE dest key [key ...], SINTERSTORE dest key [key ...], SDIFFSTORE dest key [key ...] 的解析与 handler。存储并/交/差结果到目标键。涉及文件：types.rs, parsers, handlers/set.rs, dispatch.rs, registry.rs

## 阶段 04-hash-ext: 哈希扩展命令

### 任务 4.1: HRANDFIELD 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
HRANDFIELD key [count [WITHVALUES]] 解析与 handler。从哈希中返回一个或多个随机字段。涉及文件：types.rs, parsers/hashes.rs, handlers/hash.rs, dispatch.rs, registry.rs

### 任务 4.2: HSTRLEN 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
HSTRLEN key field 解析与 handler。返回哈希字段值的字符串长度。涉及文件：types.rs, parsers/hashes.rs, handlers/hash.rs, dispatch.rs, registry.rs

## 阶段 05-bitfield: 位域操作

### 任务 5.1: BITFIELD / BITFIELD_RO 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
BITFIELD key [GET type offset] [SET type offset value] [INCRBY type offset increment] [OVERFLOW WRAP|SAT|FAIL] 和 BITFIELD_RO key [GET type offset ...] 的解析与 handler。涉及文件：types.rs, parsers/strs.rs, handlers/string.rs, dispatch.rs, registry.rs

## 阶段 06-lpos: 列表位置查找

### 任务 6.1: LPOS 实现 <!-- @depends:(none) @code_agents:1 @max_retries:3 -->
LPOS key element [RANK rank] [COUNT count] [MAXLEN len] 解析与 handler。返回列表中匹配元素的索引。涉及文件：types.rs, parsers/lists.rs, handlers/list.rs, dispatch.rs, registry.rs
