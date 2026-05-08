# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

- **Build all:** `cargo build --release`
- **Run server:** `cargo run --release` or `./your_program.sh` (listens on `127.0.0.1:6379`)
- **Run integration tests:** `cargo run --release --bin test_redis` (needs server running)
- **Filter tests by category/subcategory:** `cargo run --release --bin test_redis -- BLPOP List stream`
- **Run benchmarks:** `cargo run --release --bin stress_redis`
- **Filter benchmarks:** `cargo run --release --bin stress_redis -- large_value`
- **Launch TUI test selector:** `cargo run --release --bin tui_redis`
- **Run unit tests:** `cargo test`
- **Run single unit test:** `cargo test test_name`

## Workspace Structure

```
Cargo.toml                        # workspace root (resolver = "3")
mini-redis/                       # the Redis server implementation
test-tools/                       # test/benchmark/TUI crate
  src/
    lib.rs                        # shared logic: RedisClient, test defs, dispatch
    bin/
      test_redis.rs               # CLI integration test runner
      stress_redis.rs             # CLI benchmark runner
      tui_redis.rs                # ratatui interactive TUI
    tests/
      connection.rs               # test functions by category
      string.rs
      expiry.rs
      list.rs
      blpop.rs
      wrongtype.rs
      command.rs
      server.rs
      stream.rs
```

## Architecture

### mini-redis crate

- **`main.rs`** — TCP server entry. Protocol auto-detection per connection (first byte determines inline vs RESP mode). Spawns each connection as a separate task via `JoinSet`. Background eviction task removes expired keys every 1s.

- **`lib.rs`** — Re-exports 6 modules: `resp`, `cmd`, `db`, `inline`, `blocking`, `registry`.

- **`resp.rs`** — RESP wire protocol. `RespType` enum (`SimpleString`, `Error`, `Integer`, `BulkString`, `Array`) with `serialize() -> Vec<u8>`. Stateless `Decoder` parsing bytes into frames, returns `DecodeError::Incomplete` when more data is needed.

- **`cmd.rs`** — Command parsing + dispatch. `ParsedCmd` enum (18+ variants for all commands). `parse()` validates arguments; `dispatch_command()` routes to handler functions. Handlers access global DB via `with_db()`.

- **`db.rs`** — Global in-memory KV store: `LazyLock<Mutex<HashMap<String, Entry>>>`. `Value` enum variants: `String(Bytes)`, `List(VecDeque<Bytes>)`, `Stream(StreamData)`, `Hash`, `Set`, `ZSet` (latter 3 unused stubs). `Entry` contains `value` + optional expiry `Instant`. Access only via `with_db(|db| ...)`, never hold the Mutex lock across `.await`.

- **`inline.rs`** — Telnet inline protocol. `strip_iac()` removes Telnet negotiation bytes. `apply_backspace()` handles backspace/DEL. `parse_quoted_args()` splits by whitespace with quote support.

- **`blocking.rs`** — BLPOP blocking waiter registry. `register()` inserts `Weak<Notify>` per key, returns `BlpopGuard` (auto-unregisters on drop). `notify_waiters()` wakes live waiters.

- **`registry.rs`** — Command introspection. `init()` registers 22 commands with name/arity/category/stage. COMMAND INFO queries use `with_registry()`.

### test-tools crate

Tests and benchmarks are defined declaratively in `lib.rs` via the `tree_tests!` macro:

```rust
tree_tests! {
    "Base" => "Base" [
        _ "Connection" ["PING" => test_ping, ...]
        _ "String"     [...]
        _ "Expiry"     [...]
        _ "BLPOP"      [...]
        _ "WRONGTYPE"  [...]
        _ "Command"    [...]
        _ "Server"     [...]
    ]
    "List" => "List" [
        "Basic" => "Stages 8-16" [test_rpush_new_key, ...]
        "LRANGE"                  [...]
        "LPOP"                    [...]
    ]
    "Stream" => "Stream" [
        "XADD"   => "Stage 20"  [test_xadd_basic, ...]
        "XLEN"                   [...]
        "XRANGE"                 [...]
        ...
    ]
}
```

This generates:
- `ALL_TESTS: &[TestDef]` — flat array of all 58 test definitions with name, category, subcategory, filter, stages
- `pub async fn run_test(def: &TestDef, client: &mut RedisClient) -> Result<(), String>` — dispatch function

Actual test function implementations live in `test-tools/src/tests/` (9 modules, one per category).

**Three binary entry points:**

- **`test_redis.rs`** — Connects to running server, iterates `ALL_TESTS` with CLI filtering, prints colored PASS/FAIL. Exit code 1 if any test fails.
- **`stress_redis.rs`** — Runs selected benchmarks (throughput, large_value, many_keys, concurrent, list). Reports ops, QPS, latency.
- **`tui_redis.rs`** — Ratatui TUI with three screens: Select (tree checkboxes with expand/collapse), Running (live streaming), Results (scrollable). Runs tests directly via `tokio::runtime::Runtime::spawn` + `mpsc::channel` (no subprocess). Tab toggles functional/stress mode.

## Key Patterns

- **DB access**: `db::with_db(|db| { ... })` — never hold the Mutex across `.await`.
- **Registry access**: `registry::with_registry(|r| { ... })` — same pattern.
- **Blocking lists**: BLPOP uses `tokio::sync::Notify` + `Weak<Notify>` registry to avoid holding DB lock while waiting.
- **Protocol detection**: Per-connection, based on first byte. Sticky for connection lifetime.
- **Commands** are defined in `ParsedCmd::parse()` (string → enum) and dispatched in `dispatch_command()` (enum → handler).

## TDD Workflow

This project is a Redis implementation exercise using TDD:

1. **Write tests + scaffolding first**: When adding a new feature, implement only:
   - `ParsedCmd` enum variant
   - Command parsing in `parse()`
   - `Value` enum variant if needed
   - Data structures in `db.rs`
   - `registry.rs` entry
   - Test function in `test-tools/src/tests/`, entry in `tree_tests!` macro

2. **Handler returns stub**: New command handlers return `"ERR not implemented"`.

3. **User implements handler logic**: Making tests go from red to green.

This way the user focuses on core algorithms (stream ID generation, range queries, trimming, etc.) without setting up parsing, routing, and test infrastructure.
