# 持久化功能设计文档

## 概述
为 mini-redis 添加持久化能力：支持 SAVE/BGSAVE 命令将内存数据写入磁盘，启动时自动加载，关闭时自动保存。

## 技术方案
使用 **serde + bincode** 序列化/反序列化。对现有 `Value`、`Entry`、`StreamData` 等类型 `#[derive(Serialize, Deserialize)]`，整个 `HashMap<String, Entry>` 作为整体序列化到文件。

## 二进制格式
bincode 编码，非自定义格式，不兼容官方 RDB。文件头由 bincode 自身管理。

## 模块变更

### 新增 `mini-redis/src/persist.rs`
- `save(path: &Path) -> Result<()>` — clone DB（跳过已过期 key），bincode::serialize 写入文件
- `load(path: &Path) -> Result<HashMap<String, Entry>>` — 读取文件，bincode::deserialize
- 文件写操作使用 `std::fs`（SAVE 同步）

### Serialize/Deserialize 适配

核心问题：`Entry.expiry` 是 `Option<tokio::time::Instant>`，不可序列化。

解决方案——引入持久化专用的中间结构 `PersistEntry`：

```rust
#[derive(Serialize, Deserialize)]
struct PersistEntry {
    value: PersistValue,
    expiry_remaining_ms: Option<u64>, // 剩余毫秒数，0 = 永不过期
}
```

- **SAVE 时**：将 `Entry` 转为 `PersistEntry`，计算 `expiry_remaining_ms`（当前 Instant 到 expiry 的剩余 Duration）
- **LOAD 时**：将 `PersistEntry` 转为 `Entry`，`expiry_remaining_ms` 转为 `Instant::now() + Duration::from_millis(n)`
- SAVE 时自动跳过已过期的 key，无需持久化

对 `Value` 枚举及其子类型直接 derive Serialize/Deserialize。注意：
- `bytes::Bytes` — 需在 Cargo.toml 中启用 `serde` feature
- `HashMap<Bytes, Bytes>` — serde 原生支持
- `BTreeSet<(i64, Bytes)>` — serde 原生支持

## 配置变更 (`mini-redis/src/config.rs`)

新增字段：
- `dir: String` — 默认 `"."`  
- `dbfilename: String` — 默认 `"dump.db"`
- 组合路径：`{dir}/{dbfilename}`

CONFIG 命令扩展：
- `CONFIG SET dir <path>` — 修改保存目录
- `CONFIG SET dbfilename <name>` — 修改文件名
- `CONFIG GET dir` — 返回保存目录
- `CONFIG GET dbfilename` — 返回文件名
- `CONFIG SET` 已有 handler，扩展即可

## 命令实现

### SAVE
1. `with_db(|db| db.clone())` 快速克隆
2. 拼接路径 `{dir}/{dbfilename}`
3. `bincode::serialize(&cloned_data)` 
4. `std::fs::write(path, bytes)` 同步写入
5. 返回 `+OK` 或错误

### BGSAVE
1. `with_db(|db| db.clone())` 快速克隆
2. `tokio::spawn` 后台任务写入文件
3. 立即返回 `+OK`

## 启动流程变更 (`main.rs`)

在 `registry::init()` 之后、`bind` 之前：
1. 读取 config 的 dir + dbfilename
2. 若文件存在，反序列化加载到 DB

## 关闭流程变更 (`main.rs`)

在 `ctrl_c()` 分支中、优雅关闭之前：
1. 调用 `persist::save(path)` 自动保存

## 测试

- `test-tools/src/tests/persistence.rs` 新建测试模块
- 测试 SAVE 后文件存在且非空
- 测试 SAVE 后 restart 加载数据正确
- 测试 BGSAVE 返回 OK
- 测试 CONFIG SET dir 后 SAVE 到新位置

## 依赖

mini-redis/Cargo.toml 增加：
- `serde` (features = ["derive"])
- `bincode`
- bytes 启用 `serde` feature
