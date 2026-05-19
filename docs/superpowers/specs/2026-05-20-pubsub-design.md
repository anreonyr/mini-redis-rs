# Pub/Sub 功能设计文档

## 概述
为 mini-redis 添加 Pub/Sub 支持：PUBLISH、SUBSCRIBE、UNSUBSCRIBE。

## 架构

### `pubsub.rs` 模块
全局频道注册表：
```rust
static CHANNELS: LazyLock<Mutex<HashMap<String, Vec<UnboundedSender<Message>>>>>;
```

`Message` 结构：
```rust
pub struct Message {
    pub channel: String,
    pub payload: String,
}
```

### 订阅状态（`ConnectionState`）
```rust
pub struct SubscriptionState {
    pub channels: Vec<String>,
    pub rx: UnboundedReceiver<Message>,
}
```

### 消息流
PUBLISH 查找所有订阅者，通过 `UnboundedSender::send()` 推送消息，返回接收者数量。
SUBSCRIBE 创建 `(UnboundedSender, UnboundedReceiver)`，注册 sender，连接进入订阅模式。

### 订阅模式
SUBSCRIBE 后连接进入特殊模式：
- 正常命令处理暂停
- `tokio::select!` 同时监听 socket 输入和 pubsub 消息
- socket 输入只白名单：SUBSCRIBE/UNSUBSCRIBE/PING
- 其他命令返回 `-ERR only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT allowed`（Redis 行为）
- pubsub 消息到达时直接推送给客户端

## 命令行为

### SUBSCRIBE channel [channel ...]
- 注册所有频道到 pubsub 系统
- 推送订阅确认消息 `*3\r\n$9\r\nsubscribe\r\n$<len>\r\n<channel>\r\n:1\r\n`
- 返回订阅成功状态

### UNSUBSCRIBE [channel ...]
- 无参数时取消所有订阅
- 有参数时取消指定频道
- 推送退订确认消息

### PUBLISH channel message
- 查找频道所有订阅者
- 向每个 sender 发送消息
- 返回接收到消息的订阅者数量
- 不阻塞（使用 UnboundedSender）

## 文件变更

| 文件 | 变更 |
|------|------|
| `mini-redis/src/pubsub.rs` | 新增：全局 CHANNELS 注册表 + publish/register/unregister |
| `mini-redis/src/cmd/types.rs` | ParsedCmd 加 Subscribe/Unsubscribe/Publish |
| `mini-redis/src/cmd/parse.rs` | 解析 3 个命令 |
| `mini-redis/src/cmd/dispatch.rs` | 路由到 handler |
| `mini-redis/src/cmd/handlers/connection.rs` | handler 实现 |
| `mini-redis/src/cmd/auth.rs` | ConnectionState 加 SubscriptionState |
| `mini-redis/src/registry.rs` | 注册 3 个命令 |
| `mini-redis/src/main.rs` | 连接处理支持订阅模式 |
| `mini-redis/src/lib.rs` | 加 pub mod pubsub |

## 测试
- PUBLISH 发送消息，验证返回 0（无订阅者）
- SUBSCRIBE 后 PUBLISH，验证收到消息
- UNSUBSCRIBE 后 PUBLISH，验证不再收到
- 多个频道订阅
