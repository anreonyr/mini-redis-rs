# Pub/Sub 功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 mini-redis 添加 PUBLISH/SUBSCRIBE/UNSUBSCRIBE 命令

**Architecture:** 全局 `HashMap<Channel, Vec<UnboundedSender>>` 管理订阅。SUBSCRIBE 后连接进入"订阅模式"，`select!` 同时监听 socket 输入和 pubsub 消息。

**Tech Stack:** Rust, tokio::sync::mpsc::unbounded_channel

---

### Task 1: pubsub.rs 模块

**Files:**
- Create: `mini-redis/src/pubsub.rs`
- Modify: `mini-redis/src/lib.rs`

- [ ] **Step 1: 创建 pubsub.rs**

```rust
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// A pub/sub message pushed to all subscribers of a channel.
#[derive(Clone, Debug)]
pub struct Message {
    pub channel: String,
    pub payload: String,
}

static CHANNELS: LazyLock<Mutex<HashMap<String, Vec<UnboundedSender<Message>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Register a sender for the given channels. Returns the number of channels subscribed.
pub fn subscribe(sender: UnboundedSender<Message>, channels: &[String]) -> usize {
    let mut map = CHANNELS.lock().unwrap();
    for channel in channels {
        map.entry(channel.clone()).or_default().push(sender.clone());
    }
    channels.len()
}

/// Unregister a sender from the given channels. If channels is empty, unregister from all.
pub fn unsubscribe(sender: &UnboundedSender<Message>, channels: &[String]) -> usize {
    let mut map = CHANNELS.lock().unwrap();
    if channels.is_empty() {
        let count = map.values().map(|v| v.len()).sum();
        map.clear();
        return count;
    }
    let mut count = 0;
    for channel in channels {
        if let Some(senders) = map.get_mut(channel) {
            let before = senders.len();
            senders.retain(|s| !s.same_channel(sender));
            count += before - senders.len();
            if senders.is_empty() {
                map.remove(channel);
            }
        }
    }
    count
}

/// Publish a message to all subscribers of a channel. Returns the number of recipients.
pub fn publish(channel: &str, payload: &str) -> usize {
    let mut map = CHANNELS.lock().unwrap();
    let Some(senders) = map.get_mut(channel) else {
        return 0;
    };
    let msg = Message {
        channel: channel.to_string(),
        payload: payload.to_string(),
    };
    // Prune dead senders while delivering
    senders.retain(|sender| sender.send(msg.clone()).is_ok());
    let count = senders.len();
    if senders.is_empty() {
        map.remove(channel);
    }
    count
}
```

- [ ] **Step 2: lib.rs 注册模块**

```rust
pub mod pubsub;
```

- [ ] **Step 3: 编译验证**

```bash
cargo build --release
```

- [ ] **Step 4: Commit**

```bash
git add mini-redis/src/pubsub.rs mini-redis/src/lib.rs
git commit -m "feat: add pubsub module with channel registry"
```

---

### Task 2: SubscriptionState + 命令骨架

**Files:**
- Modify: `mini-redis/src/cmd/auth.rs`
- Modify: `mini-redis/src/cmd/types.rs`
- Modify: `mini-redis/src/cmd/parse.rs`
- Modify: `mini-redis/src/registry.rs`
- Modify: `mini-redis/src/cmd/dispatch.rs`

- [ ] **Step 1: auth.rs — ConnectionState 加 SubscriptionState**

```rust
use crate::pubsub::Message;
use tokio::sync::mpsc::UnboundedReceiver;

/// Per-connection subscription state (only set in subscription mode).
pub struct SubscriptionState {
    pub rx: UnboundedReceiver<Message>,
}

pub struct ConnectionState {
    authenticated: bool,
    pub transaction: Option<TransactionState>,
    pub watching: HashMap<String, u64>,
    pub subscription: Option<SubscriptionState>,
}
```

Update `ConnectionState::new()`:
```rust
pub fn new() -> Self {
    Self {
        authenticated: false,
        transaction: None,
        watching: HashMap::new(),
        subscription: None,
    }
}
```

Also add until `is_subscribed()`:
```rust
impl ConnectionState {
    pub fn is_subscribed(&self) -> bool {
        self.subscription.is_some()
    }
}
```

- [ ] **Step 2: types.rs — 添加 ParsedCmd 变体**

```rust
    Publish {
        channel: String,
        message: String,
    },
    Subscribe {
        channels: Vec<String>,
    },
    Unsubscribe {
        channels: Vec<String>,
    },
```

name() 方法：
```rust
    ParsedCmd::Publish { .. } => "PUBLISH",
    ParsedCmd::Subscribe { .. } => "SUBSCRIBE",
    ParsedCmd::Unsubscribe { .. } => "UNSUBSCRIBE",
```

- [ ] **Step 3: parse.rs — 解析**

```rust
"PUBLISH" => {
    if args.len() != 2 {
        return Err(wrong_arg_count("publish"));
    }
    let mut iter = args.into_iter();
    let channel = iter.next().unwrap();
    let message = iter.next().unwrap();
    ParsedCmd::Publish { channel, message }
}
"SUBSCRIBE" => {
    if args.is_empty() {
        return Err(wrong_arg_count("subscribe"));
    }
    ParsedCmd::Subscribe { channels: args }
}
"UNSUBSCRIBE" => {
    ParsedCmd::Unsubscribe { channels: args }
}
```

- [ ] **Step 4: registry.rs — 注册**

```rust
reg.register(CommandInfo {
    name: "PUBLISH",
    arity: 3,
    category: "PubSub",
    since_stage: 0,
    summary: "Posts a message to a channel",
});
reg.register(CommandInfo {
    name: "SUBSCRIBE",
    arity: -2,
    category: "PubSub",
    since_stage: 0,
    summary: "Subscribes to one or more channels",
});
reg.register(CommandInfo {
    name: "UNSUBSCRIBE",
    arity: -1,
    category: "PubSub",
    since_stage: 0,
    summary: "Unsubscribes from one or more channels",
});
```

- [ ] **Step 5: dispatch.rs — 路由**

```rust
    ParsedCmd::Publish { channel, message } => {
        handlers::handle_publish(&channel, &message)
    }
    ParsedCmd::Subscribe { channels } => {
        handlers::handle_subscribe(state, &channels)
    }
    ParsedCmd::Unsubscribe { channels } => {
        handlers::handle_unsubscribe(state, &channels)
    }
```

- [ ] **Step 6: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过（handler 暂缺，但可以先编译检查 dispatch 错误）

- [ ] **Step 7: Commit**

```bash
git add mini-redis/src/cmd/auth.rs mini-redis/src/cmd/types.rs mini-redis/src/cmd/parse.rs mini-redis/src/cmd/registry.rs mini-redis/src/cmd/dispatch.rs
git commit -m "feat: add pubsub command scaffolding and subscription state"
```

---

### Task 3: Handler 实现

**Files:**
- Modify: `mini-redis/src/cmd/handlers/connection.rs`
- Modify: `mini-redis/src/cmd/handlers/mod.rs`

- [ ] **Step 1: connection.rs — 添加 handler**

在 connection.rs 添加导入：
```rust
use crate::pubsub;
use tokio::sync::mpsc::unbounded_channel;
```

添加 handler：

```rust
pub fn handle_publish(channel: &str, message: &str) -> RespType {
    let count = pubsub::publish(channel, message);
    RespType::Integer(count as i64)
}

pub fn handle_subscribe(state: &mut ConnectionState, channels: &[String]) -> RespType {
    let (tx, rx) = unbounded_channel();
    let count = pubsub::subscribe(tx, channels);

    // Store receiver so we can enter subscription mode
    state.subscription = Some(crate::cmd::auth::SubscriptionState { rx });

    // Send subscribe confirmation messages
    // Format: *3\r\n$9\r\nsubscribe\r\n$<clen>\r\n<channel>\r\n:<count>\r\n
    // This is a multi-line response, send as array of arrays
    let mut results = Vec::new();
    for (i, ch) in channels.iter().enumerate() {
        results.push(RespType::Array(Some(vec![
            RespType::BulkString(Some(Bytes::copy_from_slice(b"subscribe"))),
            RespType::BulkString(Some(Bytes::copy_from_slice(ch.as_bytes()))),
            RespType::Integer((i + 1) as i64),
        ])));
    }
    // Return the last confirmation (Redis returns one per channel)
    results.into_iter().last().unwrap_or(
        RespType::Array(Some(vec![
            RespType::BulkString(Some(Bytes::copy_from_slice(b"subscribe"))),
            RespType::BulkString(Some(Bytes::from_static(b"0"))),
            RespType::Integer(0),
        ]))
    )
}

pub fn handle_unsubscribe(state: &mut ConnectionState, channels: &[String]) -> RespType {
    let count = pubsub::unsubscribe(channels);
    if channels.is_empty() {
        state.subscription = None;
    }

    let mut results = Vec::new();
    let target_channels = if channels.is_empty() { vec!["0".to_string()] } else { channels.to_vec() };
    for (i, ch) in target_channels.iter().enumerate() {
        results.push(RespType::Array(Some(vec![
            RespType::BulkString(Some(Bytes::copy_from_slice(b"unsubscribe"))),
            RespType::BulkString(Some(Bytes::copy_from_slice(ch.as_bytes()))),
            RespType::Integer(0),
        ])));
    }
    results.into_iter().last().unwrap_or(
        RespType::Array(Some(vec![
            RespType::BulkString(Some(Bytes::copy_from_slice(b"unsubscribe"))),
            RespType::BulkString(Some(Bytes::from_static(b"0"))),
            RespType::Integer(0),
        ]))
    )
}
```

Note: The handler values need to be sent immediately to the client. In subscription mode, the main.rs event loop sends them. The subscribe/unsubscribe handlers simply set up the state and return a response that the main loop can interpret.

Actually, for subscribe, Redis sends confirmation messages immediately. Since we return from handle_subscribe and main.rs sends the response, this works for the initial confirmation. For ongoing subscription mode, main.rs handles the separate loop.

- [ ] **Step 2: 编译验证**

```bash
cargo build --release
```

- [ ] **Step 3: Commit**

```bash
git add mini-redis/src/cmd/handlers/connection.rs
git commit -m "feat: add PUBLISH/SUBSCRIBE/UNSUBSCRIBE handlers"
```

---

### Task 4: 订阅模式主循环 + 测试

**Files:**
- Modify: `mini-redis/src/main.rs`
- Create: `test-tools/src/tests/pubsub.rs`
- Modify: `test-tools/src/tests/mod.rs`
- Modify: `test-tools/src/lib.rs`

- [ ] **Step 1: main.rs — 订阅模式支持**

订阅模式是 Pub/Sub 最复杂的部分。SUBSCRIBE 后连接进入特殊循环，用 `select!` 同时监听 socket 和 pubsub 消息。

在 `handle_connection` 中，dispatch_command 之后，如果 `state.is_subscribed()`，进入订阅模式循环：

```rust
async fn handle_connection(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let decoder = resp::Decoder::new();
    let mut read_buf = [0u8; 8192];
    let mut pending = Vec::new();
    let mut inline_mode = false;
    let mut state = cmd::ConnectionState::new();

    loop {
        if !state.is_subscribed() {
            // Normal mode: read from socket, process commands
            let n = stream.read(&mut read_buf).await
                .context("failed to read from stream")?;
            if n == 0 { return Ok(()); }

            if pending.is_empty() {
                inline_mode = !matches!(read_buf[0], b'+' | b'-' | b':' | b'$' | b'*');
            }
            pending.extend_from_slice(&read_buf[..n]);

            if pending.len() > 1024 * 1024 {
                pending.clear();
                let err = resp::RespType::Error("ERR inline buffer too large".to_string());
                send_response(&mut stream, &err).await?;
                continue;
            }

            if inline_mode {
                process_inline(&mut pending, &mut stream, &mut state).await?;
            } else {
                process_resp(&decoder, &mut pending, &mut stream, &mut state).await?;
            }

            // After successful SUBSCRIBE, switch to subscription mode
            if state.is_subscribed() {
                // Send the subscribe confirmation via process_resp already did
                // Now clear pending to start fresh in subscription mode
                pending.clear();
            }
        } else {
            // Subscription mode: select! between socket and pubsub
            use tokio::select;
            use crate::pubsub::Message;

            let mut rx = state.subscription.as_mut().unwrap().rx;

            select! {
                result = stream.read(&mut read_buf) => {
                    let n = result.context("failed to read from stream")?;
                    if n == 0 { return Ok(()); }

                    pending.extend_from_slice(&read_buf[..n]);

                    // Try to decode and handle subscription commands
                    loop {
                        match decoder.decode(&pending) {
                            Ok((frame, consumed)) => {
                                pending.drain(..consumed);
                                if let Some(cmd) = cmd::parse_command(&frame) {
                                    let cmd_name = cmd.as_ref().ok().map(|c| c.name().to_string());
                                    let is_allowed = matches!(
                                        cmd_name.as_deref(),
                                        Some("SUBSCRIBE" | "UNSUBSCRIBE" | "PING")
                                    );
                                    if is_allowed {
                                        let response = cmd::dispatch_command(cmd, &mut state).await;
                                        send_response(&mut stream, &response).await?;
                                        // If unsubscribed from all, switch back
                                        if !state.is_subscribed() {
                                            break;
                                        }
                                    } else {
                                        let err = RespType::Error(
                                            "ERR only (P)SUBSCRIBE / (P)UNSUBSCRIBE / PING / QUIT allowed in this context".to_string()
                                        );
                                        send_response(&mut stream, &err).await?;
                                    }
                                }
                            }
                            Err(resp::DecodeError::Incomplete) => break,
                            Err(resp::DecodeError::Invalid(e)) => {
                                eprintln!("decode error in sub mode: {}", e);
                                return Ok(());
                            }
                        }
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            // Push message to client as:
                            // *3\r\n$7\r\nmessage\r\n$<clen>\r\n<channel>\r\n$<plen>\r\n<payload>\r\n
                            let response = RespType::Array(Some(vec![
                                RespType::BulkString(Some(Bytes::copy_from_slice(b"message"))),
                                RespType::BulkString(Some(Bytes::copy_from_slice(msg.channel.as_bytes()))),
                                RespType::BulkString(Some(Bytes::copy_from_slice(msg.payload.as_bytes()))),
                            ]));
                            send_response(&mut stream, &response).await?;
                        }
                        None => {
                            // Channel closed, connection should unsubscribe
                            state.subscription = None;
                        }
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 2: 编译验证**

```bash
cargo build --release
```
Expected: 编译通过

- [ ] **Step 3: 创建测试**

`test-tools/src/tests/pubsub.rs`:
```rust
use crate::helpers;
use crate::RedisClient;

pub async fn test_publish_no_subscribers(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["PUBLISH", "test:channel", "hello"]).await?;
    crate::assert_resp!(resp, helpers::int(0), "PUBLISH with no subscribers should return 0");
    Ok(())
}
```

- [ ] **Step 4: tests/mod.rs + lib.rs 添加测试注册**

```rust
pub mod pubsub;
```

tree_tests! 添加：
```rust
    ("PubSub", "PubSub") [
        ("PUBLISH", "New") [
            "PUBLISH no subscribers" => tests::pubsub::test_publish_no_subscribers,
        ],
    ],
```

- [ ] **Step 5: 编译验证**

```bash
cargo build --release
```

- [ ] **Step 6: Commit**

```bash
git add mini-redis/src/main.rs test-tools/src/tests/pubsub.rs test-tools/src/tests/mod.rs test-tools/src/lib.rs
git commit -m "feat: add subscription mode loop and pubsub tests"
```

---

### 验收标准

1. `cargo build --release` 编译通过
2. PUBLISH 返回 0（无订阅者）
3. SUBSCRIBE 后连接进入订阅模式
4. PUBLISH 后订阅者收到消息 push
