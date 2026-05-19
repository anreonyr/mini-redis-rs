# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

- **Build all:** `cargo build --release`
- **Run server:** `cargo run --release` (listens on `127.0.0.1:6379`)
- **Run unit tests:** `cargo test -p mini-redis`
- **Run single unit test:** `cargo test -p mini-redis test_name`
- **Run integration tests:** `cargo run --release --bin test_redis` (needs server running)
- **Filter tests by category/subcategory:** `cargo run --release --bin test_redis -- BLPOP List stream`
- **Run benchmarks:** `cargo run --release --bin stress_redis`
- **Launch TUI test selector:** `cargo run --release --bin tui_redis`

## Workspace Structure

```
Cargo.toml                        # workspace root (resolver = "3")
mini-redis/                       # the Redis server implementation
test-tools/                       # test/benchmark/TUI crate
  src/lib.rs                      # shared logic: RedisClient, tree_tests! macro
  src/bin/
    test_redis.rs                 # CLI integration test runner
    stress_redis.rs               # CLI benchmark runner
    tui_redis.rs                  # ratatui interactive TUI
  src/tests/                      # test functions by category (22 modules)
```

## mini-redis Architecture

```
src/
├── main.rs                       # TCP entry: accept loop, eviction, graceful shutdown
├── lib.rs                        # re-exports 4 modules
├── protocol/                     # wire protocol
│   ├── resp.rs                   # RESP encoding/decoding + stateless Decoder
│   └── inline.rs                 # Telnet inline protocol parsing
├── storage/                      # data storage
│   ├── db.rs                     # per-DB Mutex<HashMap>, Value enum, Entry
│   └── persist.rs                # async save/load via spawn_blocking (30s timeout)
├── server/                       # infrastructure
│   ├── config.rs                 # LazyLock<Mutex<ServerConfig>>
│   ├── shutdown.rs               # AtomicBool graceful shutdown signal
│   ├── registry.rs               # LazyLock<Mutex<CommandRegistry>> for COMMAND INFO
│   ├── pubsub.rs                 # channel→UnboundedSender broadcast
│   └── waiters.rs                # BLPOP: Weak<Notify> registry + RAII BlpopGuard
└── cmd/                          # command system (40 files)
    ├── types.rs                  # ParsedCmd enum (110+ variants), CmdError
    ├── parse.rs                  # thin dispatch → cmd/parsers/*
    ├── dispatch.rs               # auth check + tx queue + route to handlers
    ├── auth.rs                   # ConnectionState (db_index, auth, tx, subscription)
    ├── parsers/                  # 7 per-category parse modules
    │   strs lists streams hashes sets zsets admin
    └── handlers/                 # 14 per-category handler modules
        string list stream hash set zset
        key expiry connection geo bitmap scan
```

## Key Patterns

- **DB access**: `storage::db::with_db(|db| { ... })` — never hold the Mutex across `.await`. Each DB has its own Mutex; different DBs don't contend.
- **Mutex poisoning**: All `lock().unwrap()` use `unwrap_or_else(|e| e.into_inner())` to recover from handler panics.
- **Task-local DB index**: `tokio::task_local!(DB_INDEX)` — every connection task has its own DB index. SELECT sets it; with_db() reads it. Background tasks must be wrapped in `DB_INDEX.scope(Cell::new(0), ...)`.
- **Registry access**: `server::registry::with_registry(|r| { ... })` — same closure pattern.
- **Blocking lists**: BLPOP uses `Weak<Notify>` registry + `BlpopGuard` (auto-unregisters on drop). `notify_waiters()` is called by RPUSH/LPUSH.
- **Protocol detection**: Per-connection, based on first byte. Sticky for connection lifetime.
- **Graceful shutdown**: `server::shutdown::request()` sets an AtomicBool. Accept loop checks it and stops. Active connections drain via JoinSet. Persistence saves before exit.
- **Persist**: `spawn_blocking` for blocking IO (bincode serialize + fs write), with 30-second timeout.
- **ParsedCmd dispatch**: `parse.rs` tries each category parser via `try_parser!` macro — only `CmdError::UnknownCommand` passes through; other errors (SyntaxError, WrongArgCount, InvalidInteger) propagate immediately.

## Adding a New Command

1. Add variant to `ParsedCmd` enum in `cmd/types.rs` + `.name()` match arm
2. Add parse logic in the appropriate `cmd/parsers/*.rs` module
3. Add dispatch arm in `cmd/dispatch.rs`
4. Implement handler function in the appropriate `cmd/handlers/*.rs`
5. Register in `server/registry.rs::init()`
6. Add test function in `test-tools/src/tests/` and entry in the `tree_tests!` macro in `test-tools/src/lib.rs`
