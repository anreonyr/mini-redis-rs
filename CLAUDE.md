# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

- **Build:** `cargo build --release`
- **Run locally:** `cargo run` or `./your_program.sh` (listens on `127.0.0.1:6379`)
- **Run tests:** `cargo test`
- **Run a single test:** `cargo test test_name`

## Architecture

A toy Redis clone implementing the Redis Serialization Protocol (RESP). Built with tokio for async TCP handling.

### Source layout

- **`src/main.rs`** — TCP server on port 6379. Accepts connections, reads from streams, dispatches to inline or RESP protocol handlers. Spawns a background task that evicts expired keys every 1 second.

- **`src/resp.rs`** — RESP wire protocol encode/decode. Defines `RespType` enum (`SimpleString`, `Error`, `Integer`, `BulkString`, `Array`) with `serialize()` and `Display`. The `Decoder` parses bytes into `RespType` frames; returns `DecodeError::Incomplete` when more data is needed.

- **`src/cmd.rs`** — Command parsing (`parse_command` extracts cmd+args from RESP arrays) and dispatch (`dispatch_command` routes to handlers). Supports: `PING`, `ECHO`, `SET` (with `PX`/`EX` expiry flags), `GET`, `RPUSH`, and partial `LRANGE`.

- **`src/db.rs`** — Global in-memory key-value store. A `LazyLock<Mutex<HashMap<String, Entry>>>` where `Entry` contains a `Value` (either `String(Vec<u8>)` or `List(Vec<Vec<u8>>)`) and an optional expiry `Instant`. The `with_db()` closure helper provides controlled access to the global DB.
