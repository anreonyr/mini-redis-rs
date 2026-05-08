# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

- **Build all:** `cargo build --release`
- **Run server:** `cargo run --release` or `./your_program.sh` (listens on `127.0.0.1:6379`)
- **Run feature tests:** `cargo run --release --bin test_redis` (needs server running)
- **Run stress test:** `cargo run --release --bin stress_redis`
- **Run TUI test selector:** `cargo run --release --bin tui_redis`
- **Run unit tests:** `cargo test`
- **Run a single unit test:** `cargo test test_name`

## Workspace Structure

```
Cargo.toml                  # workspace root (pure workspace, resolver = "3")
mini-redis/                 # mini-redis crate — the Redis server
test-tools/                 # test-tools crate — test/benchmark/tui binaries
```

### mini-redis crate (`mini-redis`)

- **`main.rs`** — TCP server entry point. Binds `127.0.0.1:6379`, accepts connections in a loop, spawns one task per connection. Also spawns a background eviction task that removes expired keys every 1 second.

- **`lib.rs`** — Re-exports 6 public modules: `resp`, `cmd`, `db`, `inline`, `blocking`, `registry`.

- **`resp.rs`** — RESP wire protocol. `RespType` enum (`SimpleString`, `Error`, `Integer`, `BulkString`, `Array`) with `serialize() -> Vec<u8>` and `Display`. `Decoder` parses bytes into complete frames, returns `DecodeError::Incomplete` when more data is needed. This is the only module used by the test-tools crate.

- **`cmd.rs`** — Command parsing + dispatch. `ParsedCmd` enum with variants for each command. `parse_command()` extracts cmd+args from a `RespType::Array`. `dispatch_command()` routes to handler functions. Each handler reads/writes the global DB via `with_db()`.

- **`db.rs`** — Global in-memory KV store: `LazyLock<Mutex<HashMap<String, Entry>>>`. `Value` enum has 5 variants: `String(Bytes)`, `List(VecDeque<Bytes>)`, `Hash`, `Set`, `ZSet` (latter 3 unused stubs). `Entry` contains `value` + optional expiry `Instant`. Access via `with_db()` closure helper.

- **`inline.rs`** — Telnet inline protocol support. `strip_iac()` removes Telnet negotiation bytes (0xFF). `apply_backspace()` handles backspace (0x08) and DEL (0x7F). `find_line()` locates line delimiters (\r\n or \n). `parse_quoted_args()` splits by whitespace respecting quoted strings.

- **`blocking.rs`** — BLPOP blocking waiter registry. `register()` inserts a `Weak<Notify>` per key, returns a `BlpopGuard` that auto-unregisters on drop. `notify_waiters()` wakes all live waiters for a key and cleans stale entries.

- **`registry.rs`** — Command introspection registry. `CommandRegistry` with `HashMap<String, CommandInfo>`. `init()` registers 12 commands (PING through FLUSHDB) with name, arity, category, stage. `COMMAND` query is handled via `with_registry()`.

### test-tools crate

- **`test_redis.rs`** — 39 integration tests across 6 categories (Connection, String, Expiry, List, BLPOP, WRONGTYPE). Uses a `RedisClient` struct wrapping `TcpStream` + `Decoder`. Tests connect to localhost:6379, send RESP commands, assert responses. Accepts CLI filters: `cargo run --bin test_redis -- BLPOP List`.

- **`stress_redis.rs`** — Benchmarks: SET+GET throughput, large values (1KB-100KB), many keys, concurrent connections, large list RPUSH+LRANGE. Reports QPS and latency.

- **`tui_redis.rs`** — ratatui-based interactive test selector. Three screens: Select (checkbox list), Running (live streaming output via mpsc background thread), Results (scrollable). Spawns `test_redis` as a subprocess.

## Key Patterns

- **All DB access** goes through `db::with_db(|db| { ... })` — never hold the Mutex lock across `.await` points.
- **Commands** are defined in `ParsedCmd::parse()` (string → enum) and dispatched in `dispatch_command()` (enum → handler function).
- **Communication protocol**: two modes auto-detected per connection — inline (telnet-style, `inline.rs`) and RESP (binary, `resp.rs`).
- **BLPOP blocking** uses `tokio::sync::Notify` + `Weak<Notify>` registry to avoid holding the DB lock while waiting.
