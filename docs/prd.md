# PRD: P1 级 Redis 命令补全

## 背景
P0 命令已实现完毕。P1 级命令是中等复杂度但在生产环境中常用的命令，覆盖 List 移动操作、Sorted Set 阻塞弹出、Set 存储操作、Hash 扩展、Bitmap 位域和 List 位置查找。

## 目标
实现 14 个 P1 命令，覆盖 List、ZSet、Set、Hash、Bitmap 五个类别。

## 功能列表
1. **BRPOPLPUSH** — 阻塞式 RPOPLPUSH
2. **LMOVE / BLMOVE** — Redis 6.2+ 原子列表移动（阻塞/非阻塞）
3. **ZPOPMIN / ZPOPMAX** — 有序集合弹出最低/最高分元素
4. **BZPOPMIN / BZPOPMAX** — 阻塞式有序集合弹出
5. **SUNIONSTORE / SINTERSTORE / SDIFFSTORE** — 集合并/交/差存储
6. **HRANDFIELD / HSTRLEN** — 哈希随机字段和字段长度
7. **BITFIELD / BITFIELD_RO** — 位域原子操作（GET/SET/INCRBY）
8. **LPOS** — 列表元素位置查找

## 范围
- **在此范围内：** 上述命令的完整实现（解析、分发、处理、注册、测试）
- **不在范围内：** 性能优化、ACL 权限集成

## 非功能需求
- 正确性：行为与 Redis 官方文档一致
- 可维护性：严格遵循现有项目模式

## 验收标准
1. 每个命令有正确的 RESP 响应格式
2. 边界条件与 Redis 官方行为一致
3. 所有命令注册到 COMMAND INFO
4. 集成测试覆盖正常路径和主要边界条件
