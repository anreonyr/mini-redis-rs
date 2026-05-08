use codecrafters_redis::resp::{Decoder, DecodeError, RespType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// ── ANSI helpers ──────────────────────────────────────────────────────────

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// ── RedisClient ────────────────────────────────────────────────────────────

struct RedisClient {
    stream: TcpStream,
    read_buf: Vec<u8>,
    decoder: Decoder,
    dead: bool,
}

impl RedisClient {
    async fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("IO: failed to connect to {}: {}", addr, e))?;
        Ok(Self {
            stream,
            read_buf: Vec::with_capacity(8192),
            decoder: Decoder::new(),
            dead: false,
        })
    }

    async fn cmd(&mut self, args: &[&str]) -> Result<RespType, String> {
        // Build RESP array
        let items: Vec<RespType> = args
            .iter()
            .map(|a| RespType::BulkString(Some(bytes::Bytes::copy_from_slice(a.as_bytes()))))
            .collect();
        let request = RespType::Array(Some(items));

        // Send
        if let Err(e) = self.stream.write_all(&request.serialize()).await {
            self.dead = true;
            return Err(format!("IO: write error: {}", e));
        }

        // Read and decode response
        loop {
            let mut buf = [0u8; 8192];
            let n = match self.stream.read(&mut buf).await {
                Ok(0) => {
                    self.dead = true;
                    return Err("IO: connection closed by server".to_string());
                }
                Ok(n) => n,
                Err(e) => {
                    self.dead = true;
                    return Err(format!("IO: read error: {}", e));
                }
            };
            self.read_buf.extend_from_slice(&buf[..n]);

            match self.decoder.decode(&self.read_buf) {
                Ok((frame, consumed)) => {
                    self.read_buf.drain(..consumed);
                    return Ok(frame);
                }
                Err(DecodeError::Incomplete) => {
                    if self.read_buf.len() > 1024 * 1024 {
                        self.read_buf.clear();
                        return Err("response buffer exceeded 1MB".to_string());
                    }
                    continue;
                }
                Err(DecodeError::Invalid(e)) => {
                    self.read_buf.clear();
                    return Err(format!("decode error: {}", e));
                }
            }
        }
    }
}

// ── Test result ────────────────────────────────────────────────────────────

struct TestResult {
    name: &'static str,
    category: &'static str,
    passed: bool,
    detail: Option<String>,
}

impl TestResult {
    fn fail(name: &'static str, category: &'static str, detail: String) -> Self {
        Self {
            name,
            category,
            passed: false,
            detail: Some(detail),
        }
    }

    fn pass(name: &'static str, category: &'static str) -> Self {
        Self {
            name,
            category,
            passed: true,
            detail: None,
        }
    }
}

// ── Utility ────────────────────────────────────────────────────────────────

fn simple_str(expected: &str) -> RespType {
    RespType::SimpleString(expected.to_string())
}

fn bulk_str(expected: &str) -> RespType {
    RespType::BulkString(Some(bytes::Bytes::copy_from_slice(expected.as_bytes())))
}

fn null_bulk() -> RespType {
    RespType::BulkString(None)
}

fn int(n: i64) -> RespType {
    RespType::Integer(n)
}

fn null_array() -> RespType {
    RespType::Array(None)
}

fn empty_array() -> RespType {
    RespType::Array(Some(vec![]))
}

fn arr_of_bulks(values: &[&str]) -> RespType {
    RespType::Array(Some(
        values
            .iter()
            .map(|v| RespType::BulkString(Some(bytes::Bytes::copy_from_slice(v.as_bytes()))))
            .collect(),
    ))
}

macro_rules! assert_resp {
    ($got:expr, $expected:expr, $msg:expr) => {
        if $got != $expected {
            return Err(format!(
                "{}: expected {}, got {}",
                $msg,
                $expected.to_string(),
                $got.to_string()
            ));
        }
    };
}

macro_rules! assert_match {
    ($got:expr, $pattern:pat, $msg:expr) => {
        match &$got {
            $pattern => {}
            other => {
                return Err(format!(
                    "{}: unexpected response: {}",
                    $msg,
                    other.to_string()
                ));
            }
        }
    };
}

// ── Test functions ─────────────────────────────────────────────────────────

const CONNECTION: &str = "Connection";
const STRING: &str = "String";
const EXPIRY: &str = "Expiry";
const LIST: &str = "List";
const BLPOP: &str = "BLPOP";
const WRONGTYPE: &str = "WRONGTYPE";

// ---- Connection ----

async fn test_ping(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["PING"]).await?;
    assert_resp!(r, simple_str("PONG"), "PING");
    Ok(())
}

async fn test_echo_simple(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ECHO", "hello"]).await?;
    assert_resp!(r, bulk_str("hello"), "ECHO simple");
    Ok(())
}

async fn test_echo_spaces(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["ECHO", "hello world"]).await?;
    assert_resp!(r, bulk_str("hello world"), "ECHO spaces");
    Ok(())
}

async fn test_unknown_command(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["FOOBAR"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("unknown") => Ok(()),
        _ => Err(format!("Unknown command: expected Error, got {}", r)),
    }
}

// ---- String ----

async fn test_set_get_roundtrip(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:val1", "value1"]).await?;
    assert_resp!(r, simple_str("OK"), "SET basic");
    let r = client.cmd(&["GET", "test_rs:val1"]).await?;
    assert_resp!(r, bulk_str("value1"), "GET basic");
    Ok(())
}

async fn test_get_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["GET", "test_rs:nonexist"]).await?;
    assert_resp!(r, null_bulk(), "GET nonexistent");
    Ok(())
}

async fn test_set_overwrite(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "test_rs:val1", "newval"]).await?;
    let r = client.cmd(&["GET", "test_rs:val1"]).await?;
    assert_resp!(r, bulk_str("newval"), "SET overwrite");
    Ok(())
}

async fn test_set_with_ex(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "test_rs:exkey", "val", "EX", "7200"])
        .await?;
    assert_resp!(r, simple_str("OK"), "SET EX");
    let r = client.cmd(&["GET", "test_rs:exkey"]).await?;
    assert_resp!(r, bulk_str("val"), "GET after SET EX");
    Ok(())
}

async fn test_set_with_px(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "test_rs:pxkey", "val", "PX", "7200000"])
        .await?;
    assert_resp!(r, simple_str("OK"), "SET PX");
    let r = client.cmd(&["GET", "test_rs:pxkey"]).await?;
    assert_resp!(r, bulk_str("val"), "GET after SET PX");
    Ok(())
}

async fn test_set_wrong_args(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "key"]).await?;
    assert_match!(r, RespType::Error(_), "SET wrong args");
    Ok(())
}

async fn test_set_invalid_flag(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "k", "v", "XX", "100"])
        .await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("syntax") => Ok(()),
        _ => Err(format!("SET invalid flag: expected syntax error, got {}", r)),
    }
}

async fn test_set_invalid_expiry(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "k", "v", "EX", "abc"])
        .await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("not an integer") => Ok(()),
        _ => Err(format!(
            "SET invalid expiry: expected 'not an integer', got {}",
            r
        )),
    }
}

async fn test_set_empty_value(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SET", "test_rs:empty", ""]).await?;
    assert_resp!(r, simple_str("OK"), "SET empty value");
    let r = client.cmd(&["GET", "test_rs:empty"]).await?;
    assert_resp!(r, bulk_str(""), "GET empty value");
    Ok(())
}

async fn test_set_binary_data(client: &mut RedisClient) -> Result<(), String> {
    // Use raw bytes in args — cmd() sends them as bulk strings
    let key = "test_rs:bin";
    let value = "value_with_null_\x00_and_ff_\u{ff}";
    let r = client.cmd(&["SET", key, value]).await?;
    assert_resp!(r, simple_str("OK"), "SET binary");
    let r = client.cmd(&["GET", key]).await?;
    match &r {
        RespType::BulkString(Some(data)) => {
            if data[..] == value.as_bytes()[..] {
                Ok(())
            } else {
                Err(format!("GET binary: data mismatch, got {:?}", data))
            }
        }
        _ => Err(format!("GET binary: expected BulkString, got {}", r)),
    }
}

// ---- Expiry ----

async fn test_ex_actual_expiry(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "test_rs:exp_ex", "val", "EX", "1"])
        .await?;
    assert_resp!(r, simple_str("OK"), "SET EX 1");

    // Should still exist immediately
    let r = client.cmd(&["GET", "test_rs:exp_ex"]).await?;
    assert_resp!(r, bulk_str("val"), "GET before expiry");

    // Wait for it to expire
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let r = client.cmd(&["GET", "test_rs:exp_ex"]).await?;
    assert_resp!(r, null_bulk(), "GET after EX expiry");
    Ok(())
}

async fn test_px_actual_expiry(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "test_rs:exp_px", "val", "PX", "500"])
        .await?;
    assert_resp!(r, simple_str("OK"), "SET PX 500");

    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;

    let r = client.cmd(&["GET", "test_rs:exp_px"]).await?;
    assert_resp!(r, null_bulk(), "GET after PX expiry");
    Ok(())
}

async fn test_expiry_background_cleanup(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["SET", "test_rs:exp_bg", "val", "EX", "1"])
        .await?;
    assert_resp!(r, simple_str("OK"), "SET EX 1 for bg cleanup");

    // Wait 2.5s — the background task should clean it up
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    // Try LLEN on the expired key — it's a string, but it should be gone
    let r = client.cmd(&["GET", "test_rs:exp_bg"]).await?;
    assert_resp!(r, null_bulk(), "GET after background cleanup");
    Ok(())
}

// ---- List ----

async fn test_rpush_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["RPUSH", "test_rs:list", "a", "b", "c"])
        .await?;
    assert_resp!(r, int(3), "RPUSH new key");
    let r = client
        .cmd(&["LRANGE", "test_rs:list", "0", "-1"])
        .await?;
    assert_resp!(r, arr_of_bulks(&["a", "b", "c"]), "LRANGE verify");
    Ok(())
}

async fn test_rpush_existing_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["RPUSH", "test_rs:list", "d", "e"])
        .await?;
    assert_resp!(r, int(5), "RPUSH existing key");
    let r = client
        .cmd(&["LRANGE", "test_rs:list", "0", "-1"])
        .await?;
    assert_resp!(
        r,
        arr_of_bulks(&["a", "b", "c", "d", "e"]),
        "LRANGE after RPUSH"
    );
    Ok(())
}

async fn test_lpush_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LPUSH", "test_rs:list2", "x", "y"])
        .await?;
    assert_resp!(r, int(2), "LPUSH new key");
    let r = client
        .cmd(&["LRANGE", "test_rs:list2", "0", "-1"])
        .await?;
    assert_resp!(r, arr_of_bulks(&["y", "x"]), "LRANGE after LPUSH");
    Ok(())
}

async fn test_lrange_positive_indices(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LRANGE", "test_rs:list", "1", "2"])
        .await?;
    assert_resp!(r, arr_of_bulks(&["b", "c"]), "LRANGE positive indices");
    Ok(())
}

async fn test_lrange_negative_indices(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LRANGE", "test_rs:list", "-2", "-1"])
        .await?;
    assert_resp!(r, arr_of_bulks(&["d", "e"]), "LRANGE negative indices");
    Ok(())
}

async fn test_lrange_out_of_bounds(client: &mut RedisClient) -> Result<(), String> {
    // start > stop
    let r = client
        .cmd(&["LRANGE", "test_rs:list", "10", "20"])
        .await?;
    assert_resp!(r, empty_array(), "LRANGE out of bounds");
    Ok(())
}

async fn test_lrange_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LRANGE", "test_rs:nonexlist", "0", "-1"])
        .await?;
    assert_resp!(r, empty_array(), "LRANGE empty key");
    Ok(())
}

async fn test_llen(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
    assert_resp!(r, int(5), "LLEN");
    Ok(())
}

async fn test_llen_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LLEN", "test_rs:nonexlist"])
        .await?;
    assert_resp!(r, int(0), "LLEN empty key");
    Ok(())
}

async fn test_lpop_single(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:list"]).await?;
    assert_resp!(r, bulk_str("a"), "LPOP single");
    // Verify list now has 4 elements
    let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
    assert_resp!(r, int(4), "LLEN after LPOP");
    Ok(())
}

async fn test_lpop_with_count(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:list", "2"]).await?;
    assert_resp!(r, arr_of_bulks(&["b", "c"]), "LPOP with count 2");
    let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
    assert_resp!(r, int(2), "LLEN after LPOP 2");
    Ok(())
}

async fn test_lpop_count_zero(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LPOP", "test_rs:list", "0"])
        .await?;
    // Should return empty array
    match &r {
        RespType::Array(Some(items)) if items.is_empty() => Ok(()),
        _ => Err(format!("LPOP count=0: expected empty array, got {}", r)),
    }
}

async fn test_lpop_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["LPOP", "test_rs:nonexlist"])
        .await?;
    assert_resp!(r, null_bulk(), "LPOP empty key");
    Ok(())
}

async fn test_lpop_count_larger_than_list(client: &mut RedisClient) -> Result<(), String> {
    // list now has [d, e] from previous ops
    let r = client.cmd(&["LPOP", "test_rs:list", "10"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            // key should be deleted now — verify
            let r2 = client.cmd(&["LLEN", "test_rs:list"]).await?;
            assert_resp!(r2, int(0), "LLEN after LPOP count > len");
            Ok(())
        }
        _ => Err(format!(
            "LPOP count>len: expected Array of 2, got {}",
            r
        )),
    }
}

async fn test_large_list_lrange(client: &mut RedisClient) -> Result<(), String> {
    // Push 1000 elements
    let mut args: Vec<&str> = vec!["RPUSH", "test_rs:biglist"];
    let num_strs: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
    let str_refs: Vec<&str> = num_strs.iter().map(|s| s.as_str()).collect();
    args.extend(&str_refs);
    let r = client.cmd(&args).await?;
    assert_resp!(r, int(1000), "RPUSH 1000 elements");

    // Verify LRANGE returns all
    let r = client
        .cmd(&["LRANGE", "test_rs:biglist", "0", "-1"])
        .await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 1000 => Ok(()),
        _ => Err(format!(
            "LRANGE 1000: expected Array of 1000, got {}",
            r
        )),
    }
}

async fn test_list_empty_string_element(client: &mut RedisClient) -> Result<(), String> {
    let r = client
        .cmd(&["RPUSH", "test_rs:emptylist", ""])
        .await?;
    assert_resp!(r, int(1), "RPUSH empty string");
    let r = client
        .cmd(&["LPOP", "test_rs:emptylist"])
        .await?;
    assert_resp!(r, bulk_str(""), "LPOP empty string");
    Ok(())
}

// ---- BLPOP ----

async fn test_blpop_immediate(client: &mut RedisClient) -> Result<(), String> {
    // Setup data
    let _ = client
        .cmd(&["RPUSH", "test_rs:blpop_imm", "val"])
        .await?;

    let now = tokio::time::Instant::now();
    let r = client
        .cmd(&["BLPOP", "test_rs:blpop_imm", "0"])
        .await?;
    let elapsed = now.elapsed();

    // Should return immediately (< 100ms)
    if elapsed.as_millis() > 100 {
        return Err(format!(
            "BLPOP immediate: took {}ms, expected < 100ms",
            elapsed.as_millis()
        ));
    }

    // Expect Array([BulkString("test_rs:blpop_imm"), BulkString("val")])
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => Ok(()),
        _ => Err(format!(
            "BLPOP immediate: expected Array of 2, got {}",
            r
        )),
    }
}

async fn test_blpop_timeout(client: &mut RedisClient) -> Result<(), String> {
    let now = tokio::time::Instant::now();
    let r = client
        .cmd(&["BLPOP", "test_rs:blpop_empty", "1"])
        .await?;
    let elapsed = now.elapsed();

    // Should block for ~1 second
    if elapsed.as_millis() < 800 {
        return Err(format!(
            "BLPOP timeout: took {}ms, expected >= 800ms (timeout=1s)",
            elapsed.as_millis()
        ));
    }

    assert_resp!(r, null_array(), "BLPOP timeout");
    Ok(())
}

async fn test_blpop_multi_key(client: &mut RedisClient) -> Result<(), String> {
    // First key empty, second has data
    let _ = client
        .cmd(&["RPUSH", "test_rs:blpop_multi", "winner"])
        .await?;

    let r = client
        .cmd(&[
            "BLPOP",
            "test_rs:blpop_empty",
            "test_rs:blpop_multi",
            "1",
        ])
        .await?;

    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            // First element should be the key name, second the value
            if let RespType::BulkString(Some(key)) = &items[0] {
                let key_str = String::from_utf8_lossy(key);
                if key_str == "test_rs:blpop_multi" {
                    Ok(())
                } else {
                    Err(format!("BLPOP multi-key: expected test_rs:blpop_multi as key, got {}", key_str))
                }
            } else {
                Err(format!("BLPOP multi-key: unexpected format: {}", r))
            }
        }
        _ => Err(format!("BLPOP multi-key: expected Array of 2, got {}", r)),
    }
}

async fn test_blpop_wakeup(client_b: &mut RedisClient) -> Result<(), String> {
    // Client A: create a separate connection that blocks on BLPOP
    let mut client_a = RedisClient::connect("127.0.0.1:6379").await?;

    let handle_a = tokio::spawn(async move {
        let now = tokio::time::Instant::now();
        let r = client_a
            .cmd(&["BLPOP", "test_rs:blpop_wakeup", "5"])
            .await;
        (now.elapsed(), r)
    });

    // Small delay to ensure client_a is blocked
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Client B pushes data to wake client A
    let r = client_b
        .cmd(&["RPUSH", "test_rs:blpop_wakeup", "wakeup"])
        .await?;
    assert_resp!(r, int(1), "RPUSH wakeup");

    // Wait for client A to unblock
    let (elapsed, result) = handle_a.await.map_err(|e| format!("join error: {}", e))?;

    if elapsed.as_millis() > 3000 {
        return Err(format!(
            "BLPOP wakeup: took {}ms, expected fast wakeup (< 3000ms)",
            elapsed.as_millis()
        ));
    }

    match &result {
        Ok(RespType::Array(Some(items))) if items.len() == 2 => Ok(()),
        Ok(other) => Err(format!(
            "BLPOP wakeup: expected Array of 2, got {}",
            other
        )),
        Err(e) => Err(format!("BLPOP wakeup: client_a error: {}", e)),
    }
}

// ---- WRONGTYPE ----

async fn test_wrongtype_get_on_list(client: &mut RedisClient) -> Result<(), String> {
    let _ = client
        .cmd(&["RPUSH", "test_rs:wt_list", "a"])
        .await?;
    let r = client.cmd(&["GET", "test_rs:wt_list"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("GET on list: expected WRONGTYPE, got {}", r)),
    }
}

async fn test_wrongtype_llen_on_string(client: &mut RedisClient) -> Result<(), String> {
    let _ = client
        .cmd(&["SET", "test_rs:wt_str", "val"])
        .await?;
    let r = client.cmd(&["LLEN", "test_rs:wt_str"]).await?;
    match &r {
        RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
        _ => Err(format!("LLEN on string: expected WRONGTYPE, got {}", r)),
    }
}

// ── Output formatter ───────────────────────────────────────────────────────

fn print_results(results: &[TestResult]) {
    let mut current_category = "";
    for r in results {
        if r.category != current_category {
            println!("\n{BOLD}[{}]{RESET}", r.category);
            current_category = r.category;
        }
        if r.passed {
            println!("  {GREEN}[PASS]{RESET} {}", r.name);
        } else {
            println!("  {RED}[FAIL]{RESET} {}", r.name);
            if let Some(detail) = &r.detail {
                println!("          {YELLOW}{DIM}{}{RESET}", detail);
            }
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();
    let total = results.len();

    println!();
    if failed == 0 {
        println!(
            "{GREEN}{BOLD}Results: {} passed, {} failed, {} total{RESET}",
            passed, failed, total
        );
    } else {
        println!(
            "{RED}{BOLD}Results: {} passed, {} failed, {} total{RESET}",
            passed, failed, total
        );
    }
}

// ── Runner ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("{BOLD}Redis Test Runner v0.1.0{RESET}");
    println!("Target: 127.0.0.1:6379");
    println!("─────────────────────────────────────────────────");

    let mut client = match RedisClient::connect("127.0.0.1:6379").await {
        Ok(c) => {
            println!("Connected.\n");
            c
        }
        Err(e) => {
            eprintln!("{RED}FAILED to connect: {}{RESET}", e);
            std::process::exit(1);
        }
    };

    let mut results: Vec<TestResult> = Vec::new();

    // ── Connection tests ───────────────────────────────────────────────

    macro_rules! run {
        ($test_fn:expr, $name:expr, $cat:expr, $client:expr) => {{
            let res = $test_fn($client).await;
            match res {
                Ok(()) => results.push(TestResult::pass($name, $cat)),
                Err(e) => results.push(TestResult::fail($name, $cat, e)),
            }
        }};
    }

    run!(test_ping, "PING", CONNECTION, &mut client);
    run!(test_echo_simple, "ECHO simple", CONNECTION, &mut client);
    run!(test_echo_spaces, "ECHO with spaces", CONNECTION, &mut client);
    run!(test_unknown_command, "Unknown command", CONNECTION, &mut client);

    run!(test_set_get_roundtrip, "SET+GET roundtrip", STRING, &mut client);
    run!(test_get_nonexistent, "GET nonexistent key", STRING, &mut client);
    run!(test_set_overwrite, "SET overwrite", STRING, &mut client);
    run!(test_set_with_ex, "SET with EX", STRING, &mut client);
    run!(test_set_with_px, "SET with PX", STRING, &mut client);
    run!(test_set_wrong_args, "SET wrong arg count", STRING, &mut client);
    run!(test_set_invalid_flag, "SET invalid flag", STRING, &mut client);
    run!(test_set_invalid_expiry, "SET invalid expiry", STRING, &mut client);
    run!(test_set_empty_value, "SET empty value", STRING, &mut client);
    run!(test_set_binary_data, "SET binary data", STRING, &mut client);

    run!(test_ex_actual_expiry, "EX expiry actually expires", EXPIRY, &mut client);
    run!(test_px_actual_expiry, "PX expiry actually expires", EXPIRY, &mut client);
    run!(test_expiry_background_cleanup, "Background expiry cleanup", EXPIRY, &mut client);

    run!(test_rpush_new_key, "RPUSH new key", LIST, &mut client);
    run!(test_rpush_existing_key, "RPUSH existing key", LIST, &mut client);
    run!(test_lpush_new_key, "LPUSH new key", LIST, &mut client);
    run!(test_lrange_positive_indices, "LRANGE positive indices", LIST, &mut client);
    run!(test_lrange_negative_indices, "LRANGE negative indices", LIST, &mut client);
    run!(test_lrange_out_of_bounds, "LRANGE out of bounds", LIST, &mut client);
    run!(test_lrange_empty_key, "LRANGE empty key", LIST, &mut client);
    run!(test_llen, "LLEN", LIST, &mut client);
    run!(test_llen_empty_key, "LLEN empty key", LIST, &mut client);
    run!(test_lpop_single, "LPOP single", LIST, &mut client);
    run!(test_lpop_with_count, "LPOP with count", LIST, &mut client);
    run!(test_lpop_count_zero, "LPOP count=0", LIST, &mut client);
    run!(test_lpop_empty_key, "LPOP empty key", LIST, &mut client);
    run!(test_lpop_count_larger_than_list, "LPOP count > list len", LIST, &mut client);
    run!(test_large_list_lrange, "LRANGE 1000 elements", LIST, &mut client);
    run!(test_list_empty_string_element, "List empty string element", LIST, &mut client);

    run!(test_blpop_immediate, "BLPOP immediate (has data)", BLPOP, &mut client);
    run!(test_blpop_timeout, "BLPOP timeout", BLPOP, &mut client);
    run!(test_blpop_multi_key, "BLPOP multi key", BLPOP, &mut client);

    // ── BLPOP multi-connection wakeup ──────────────────────────────────
    {
        let name = "BLPOP wakeup (multi-conn)";
        let cat = BLPOP;
        match test_blpop_wakeup(&mut client).await {
            Ok(()) => results.push(TestResult::pass(name, cat)),
            Err(e) => results.push(TestResult::fail(name, cat, e)),
        }
    }

    run!(test_wrongtype_get_on_list, "GET on list -> WRONGTYPE", WRONGTYPE, &mut client);
    run!(test_wrongtype_llen_on_string, "LLEN on string -> WRONGTYPE", WRONGTYPE, &mut client);

    // ── Results ────────────────────────────────────────────────────────

    print_results(&results);

    let failed = results.iter().any(|r| !r.passed);
    std::process::exit(if failed { 1 } else { 0 });
}
