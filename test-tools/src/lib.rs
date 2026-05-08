use mini_redis::resp::{Decoder, DecodeError, RespType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// ── RedisClient ────────────────────────────────────────────────────────

pub struct RedisClient {
    stream: TcpStream,
    read_buf: Vec<u8>,
    decoder: Decoder,
    pub dead: bool,
}

impl RedisClient {
    pub async fn connect(addr: &str) -> Result<Self, String> {
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

    pub async fn cmd(&mut self, args: &[&str]) -> Result<RespType, String> {
        let items: Vec<RespType> = args
            .iter()
            .map(|a| RespType::BulkString(Some(bytes::Bytes::copy_from_slice(a.as_bytes()))))
            .collect();
        let request = RespType::Array(Some(items));

        if let Err(e) = self.stream.write_all(&request.serialize()).await {
            self.dead = true;
            return Err(format!("IO: write error: {}", e));
        }

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

// ── TestResult ─────────────────────────────────────────────────────────

pub struct TestResult {
    pub name: &'static str,
    pub category: &'static str,
    pub passed: bool,
    pub detail: Option<String>,
}

impl TestResult {
    pub fn fail(name: &'static str, category: &'static str, detail: String) -> Self {
        Self { name, category, passed: false, detail: Some(detail) }
    }

    pub fn pass(name: &'static str, category: &'static str) -> Self {
        Self { name, category, passed: true, detail: None }
    }
}

// ── BenchResult ────────────────────────────────────────────────────────

pub struct BenchResult {
    pub name: &'static str,
    pub ops: u64,
    pub elapsed_ms: u64,
    pub avg_latency_us: f64,
}

impl BenchResult {
    pub fn qps(&self) -> f64 {
        if self.elapsed_ms == 0 { return 0.0; }
        self.ops as f64 / (self.elapsed_ms as f64 / 1000.0)
    }
}

// ── RESP helpers ───────────────────────────────────────────────────────

pub mod helpers {
    use mini_redis::resp::RespType;

    pub fn simple_str(expected: &str) -> RespType {
        RespType::SimpleString(expected.to_string())
    }

    pub fn bulk_str(expected: &str) -> RespType {
        RespType::BulkString(Some(bytes::Bytes::copy_from_slice(expected.as_bytes())))
    }

    pub fn null_bulk() -> RespType {
        RespType::BulkString(None)
    }

    pub fn int(n: i64) -> RespType {
        RespType::Integer(n)
    }

    pub fn null_array() -> RespType {
        RespType::Array(None)
    }

    pub fn empty_array() -> RespType {
        RespType::Array(Some(vec![]))
    }

    pub fn arr_of_bulks(values: &[&str]) -> RespType {
        RespType::Array(Some(
            values.iter()
                .map(|v| RespType::BulkString(Some(bytes::Bytes::copy_from_slice(v.as_bytes()))))
                .collect(),
        ))
    }
}

// ── Assertion macros ───────────────────────────────────────────────────

#[macro_export]
macro_rules! assert_resp {
    ($got:expr, $expected:expr, $msg:expr) => {
        if $got != $expected {
            return Err(format!("{}: expected {}, got {}", $msg, $expected.to_string(), $got.to_string()));
        }
    };
}

#[macro_export]
macro_rules! assert_match {
    ($got:expr, $pattern:pat, $msg:expr) => {
        match &$got {
            $pattern => {}
            other => return Err(format!("{}: unexpected response: {}", $msg, other.to_string())),
        }
    };
}

// ── Functional test functions ──────────────────────────────────────────

pub mod functional_tests {
    use crate::helpers::*;
    use crate::RedisClient;
    use mini_redis::resp::RespType;

    pub async fn test_ping(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["PING"]).await?;
        crate::assert_resp!(r, simple_str("PONG"), "PING");
        Ok(())
    }

    pub async fn test_echo_simple(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["ECHO", "hello"]).await?;
        crate::assert_resp!(r, bulk_str("hello"), "ECHO simple");
        Ok(())
    }

    pub async fn test_echo_spaces(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["ECHO", "hello world"]).await?;
        crate::assert_resp!(r, bulk_str("hello world"), "ECHO spaces");
        Ok(())
    }

    pub async fn test_unknown_command(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["FOOBAR"]).await?;
        match &r {
            RespType::Error(msg) if msg.to_lowercase().contains("unknown") => Ok(()),
            _ => Err(format!("Unknown command: expected Error, got {}", r)),
        }
    }

    pub async fn test_set_get_roundtrip(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:val1", "value1"]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET basic");
        let r = client.cmd(&["GET", "test_rs:val1"]).await?;
        crate::assert_resp!(r, bulk_str("value1"), "GET basic");
        Ok(())
    }

    pub async fn test_get_nonexistent(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["GET", "test_rs:nonexist"]).await?;
        crate::assert_resp!(r, null_bulk(), "GET nonexistent");
        Ok(())
    }

    pub async fn test_set_overwrite(client: &mut RedisClient) -> Result<(), String> {
        let _ = client.cmd(&["SET", "test_rs:val1", "newval"]).await?;
        let r = client.cmd(&["GET", "test_rs:val1"]).await?;
        crate::assert_resp!(r, bulk_str("newval"), "SET overwrite");
        Ok(())
    }

    pub async fn test_set_with_ex(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:exkey", "val", "EX", "7200"]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET EX");
        let r = client.cmd(&["GET", "test_rs:exkey"]).await?;
        crate::assert_resp!(r, bulk_str("val"), "GET after SET EX");
        Ok(())
    }

    pub async fn test_set_with_px(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:pxkey", "val", "PX", "7200000"]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET PX");
        let r = client.cmd(&["GET", "test_rs:pxkey"]).await?;
        crate::assert_resp!(r, bulk_str("val"), "GET after SET PX");
        Ok(())
    }

    pub async fn test_set_wrong_args(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "key"]).await?;
        crate::assert_match!(r, RespType::Error(_), "SET wrong args");
        Ok(())
    }

    pub async fn test_set_invalid_flag(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "k", "v", "XX", "100"]).await?;
        match &r {
            RespType::Error(msg) if msg.to_lowercase().contains("syntax") => Ok(()),
            _ => Err(format!("SET invalid flag: expected syntax error, got {}", r)),
        }
    }

    pub async fn test_set_invalid_expiry(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "k", "v", "EX", "abc"]).await?;
        match &r {
            RespType::Error(msg) if msg.to_lowercase().contains("not an integer") => Ok(()),
            _ => Err(format!("SET invalid expiry: expected 'not an integer', got {}", r)),
        }
    }

    pub async fn test_set_empty_value(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:empty", ""]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET empty value");
        let r = client.cmd(&["GET", "test_rs:empty"]).await?;
        crate::assert_resp!(r, bulk_str(""), "GET empty value");
        Ok(())
    }

    pub async fn test_set_binary_data(client: &mut RedisClient) -> Result<(), String> {
        let key = "test_rs:bin";
        let value = "value_with_null_\x00_and_ff_\u{ff}";
        let r = client.cmd(&["SET", key, value]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET binary");
        let r = client.cmd(&["GET", key]).await?;
        match &r {
            RespType::BulkString(Some(data)) if data[..] == value.as_bytes()[..] => Ok(()),
            RespType::BulkString(Some(data)) => Err(format!("GET binary: data mismatch, got {:?}", data)),
            _ => Err(format!("GET binary: expected BulkString, got {}", r)),
        }
    }

    pub async fn test_ex_actual_expiry(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:exp_ex", "val", "EX", "1"]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET EX 1");
        let r = client.cmd(&["GET", "test_rs:exp_ex"]).await?;
        crate::assert_resp!(r, bulk_str("val"), "GET before expiry");
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let r = client.cmd(&["GET", "test_rs:exp_ex"]).await?;
        crate::assert_resp!(r, null_bulk(), "GET after EX expiry");
        Ok(())
    }

    pub async fn test_px_actual_expiry(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:exp_px", "val", "PX", "500"]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET PX 500");
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        let r = client.cmd(&["GET", "test_rs:exp_px"]).await?;
        crate::assert_resp!(r, null_bulk(), "GET after PX expiry");
        Ok(())
    }

    pub async fn test_expiry_background_cleanup(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["SET", "test_rs:exp_bg", "val", "EX", "1"]).await?;
        crate::assert_resp!(r, simple_str("OK"), "SET EX 1 for bg cleanup");
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        let r = client.cmd(&["GET", "test_rs:exp_bg"]).await?;
        crate::assert_resp!(r, null_bulk(), "GET after background cleanup");
        Ok(())
    }

    pub async fn test_rpush_new_key(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["RPUSH", "test_rs:list", "a", "b", "c"]).await?;
        crate::assert_resp!(r, int(3), "RPUSH new key");
        let r = client.cmd(&["LRANGE", "test_rs:list", "0", "-1"]).await?;
        crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c"]), "LRANGE verify");
        Ok(())
    }

    pub async fn test_rpush_existing_key(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["RPUSH", "test_rs:list", "d", "e"]).await?;
        crate::assert_resp!(r, int(5), "RPUSH existing key");
        let r = client.cmd(&["LRANGE", "test_rs:list", "0", "-1"]).await?;
        crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c", "d", "e"]), "LRANGE after RPUSH");
        Ok(())
    }

    pub async fn test_lpush_new_key(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LPUSH", "test_rs:list2", "x", "y"]).await?;
        crate::assert_resp!(r, int(2), "LPUSH new key");
        let r = client.cmd(&["LRANGE", "test_rs:list2", "0", "-1"]).await?;
        crate::assert_resp!(r, arr_of_bulks(&["y", "x"]), "LRANGE after LPUSH");
        Ok(())
    }

    pub async fn test_lrange_positive_indices(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LRANGE", "test_rs:list", "1", "2"]).await?;
        crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "LRANGE positive indices");
        Ok(())
    }

    pub async fn test_lrange_negative_indices(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LRANGE", "test_rs:list", "-2", "-1"]).await?;
        crate::assert_resp!(r, arr_of_bulks(&["d", "e"]), "LRANGE negative indices");
        Ok(())
    }

    pub async fn test_lrange_out_of_bounds(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LRANGE", "test_rs:list", "10", "20"]).await?;
        crate::assert_resp!(r, empty_array(), "LRANGE out of bounds");
        Ok(())
    }

    pub async fn test_lrange_empty_key(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LRANGE", "test_rs:nonexlist", "0", "-1"]).await?;
        crate::assert_resp!(r, empty_array(), "LRANGE empty key");
        Ok(())
    }

    pub async fn test_llen(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
        crate::assert_resp!(r, int(5), "LLEN");
        Ok(())
    }

    pub async fn test_llen_empty_key(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LLEN", "test_rs:nonexlist"]).await?;
        crate::assert_resp!(r, int(0), "LLEN empty key");
        Ok(())
    }

    pub async fn test_lpop_single(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LPOP", "test_rs:list"]).await?;
        crate::assert_resp!(r, bulk_str("a"), "LPOP single");
        let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
        crate::assert_resp!(r, int(4), "LLEN after LPOP");
        Ok(())
    }

    pub async fn test_lpop_with_count(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LPOP", "test_rs:list", "2"]).await?;
        crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "LPOP with count 2");
        let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
        crate::assert_resp!(r, int(2), "LLEN after LPOP 2");
        Ok(())
    }

    pub async fn test_lpop_count_zero(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LPOP", "test_rs:list", "0"]).await?;
        match &r {
            RespType::Array(Some(items)) if items.is_empty() => Ok(()),
            _ => Err(format!("LPOP count=0: expected empty array, got {}", r)),
        }
    }

    pub async fn test_lpop_empty_key(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LPOP", "test_rs:nonexlist"]).await?;
        crate::assert_resp!(r, null_bulk(), "LPOP empty key");
        Ok(())
    }

    pub async fn test_lpop_count_larger_than_list(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["LPOP", "test_rs:list", "10"]).await?;
        match &r {
            RespType::Array(Some(items)) if items.len() == 2 => {
                let r2 = client.cmd(&["LLEN", "test_rs:list"]).await?;
                crate::assert_resp!(r2, int(0), "LLEN after LPOP count > len");
                Ok(())
            }
            _ => Err(format!("LPOP count>len: expected Array of 2, got {}", r)),
        }
    }

    pub async fn test_large_list_lrange(client: &mut RedisClient) -> Result<(), String> {
        let mut args: Vec<&str> = vec!["RPUSH", "test_rs:biglist"];
        let num_strs: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
        let str_refs: Vec<&str> = num_strs.iter().map(|s| s.as_str()).collect();
        args.extend(&str_refs);
        let r = client.cmd(&args).await?;
        crate::assert_resp!(r, int(1000), "RPUSH 1000 elements");
        let r = client.cmd(&["LRANGE", "test_rs:biglist", "0", "-1"]).await?;
        match &r {
            RespType::Array(Some(items)) if items.len() == 1000 => Ok(()),
            _ => Err(format!("LRANGE 1000: expected Array of 1000, got {}", r)),
        }
    }

    pub async fn test_list_empty_string_element(client: &mut RedisClient) -> Result<(), String> {
        let r = client.cmd(&["RPUSH", "test_rs:emptylist", ""]).await?;
        crate::assert_resp!(r, int(1), "RPUSH empty string");
        let r = client.cmd(&["LPOP", "test_rs:emptylist"]).await?;
        crate::assert_resp!(r, bulk_str(""), "LPOP empty string");
        Ok(())
    }

    pub async fn test_blpop_immediate(client: &mut RedisClient) -> Result<(), String> {
        let _ = client.cmd(&["RPUSH", "test_rs:blpop_imm", "val"]).await?;
        let now = tokio::time::Instant::now();
        let r = client.cmd(&["BLPOP", "test_rs:blpop_imm", "0"]).await?;
        let elapsed = now.elapsed();
        if elapsed.as_millis() > 100 {
            return Err(format!("BLPOP immediate: took {}ms, expected < 100ms", elapsed.as_millis()));
        }
        match &r {
            RespType::Array(Some(items)) if items.len() == 2 => Ok(()),
            _ => Err(format!("BLPOP immediate: expected Array of 2, got {}", r)),
        }
    }

    pub async fn test_blpop_timeout(client: &mut RedisClient) -> Result<(), String> {
        let now = tokio::time::Instant::now();
        let r = client.cmd(&["BLPOP", "test_rs:blpop_empty", "1"]).await?;
        let elapsed = now.elapsed();
        if elapsed.as_millis() < 800 {
            return Err(format!("BLPOP timeout: took {}ms, expected >= 800ms", elapsed.as_millis()));
        }
        crate::assert_resp!(r, null_array(), "BLPOP timeout");
        Ok(())
    }

    pub async fn test_blpop_multi_key(client: &mut RedisClient) -> Result<(), String> {
        let _ = client.cmd(&["RPUSH", "test_rs:blpop_multi", "winner"]).await?;
        let r = client.cmd(&["BLPOP", "test_rs:blpop_empty", "test_rs:blpop_multi", "1"]).await?;
        match &r {
            RespType::Array(Some(items)) if items.len() == 2 => {
                if let RespType::BulkString(Some(key)) = &items[0] {
                    if String::from_utf8_lossy(key) == "test_rs:blpop_multi" {
                        return Ok(());
                    }
                }
                Err(format!("BLPOP multi-key: unexpected format: {}", r))
            }
            _ => Err(format!("BLPOP multi-key: expected Array of 2, got {}", r)),
        }
    }

    pub async fn test_blpop_wakeup(client_b: &mut RedisClient) -> Result<(), String> {
        let mut client_a = RedisClient::connect("127.0.0.1:6379").await?;
        let handle_a = tokio::spawn(async move {
            let now = tokio::time::Instant::now();
            let r = client_a.cmd(&["BLPOP", "test_rs:blpop_wakeup", "5"]).await;
            (now.elapsed(), r)
        });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let r = client_b.cmd(&["RPUSH", "test_rs:blpop_wakeup", "wakeup"]).await?;
        crate::assert_resp!(r, int(1), "RPUSH wakeup");
        let (elapsed, result) = handle_a.await.map_err(|e| format!("join error: {}", e))?;
        if elapsed.as_millis() > 3000 {
            return Err(format!("BLPOP wakeup: took {}ms, expected < 3000ms", elapsed.as_millis()));
        }
        match &result {
            Ok(RespType::Array(Some(items))) if items.len() == 2 => Ok(()),
            Ok(other) => Err(format!("BLPOP wakeup: expected Array of 2, got {}", other)),
            Err(e) => Err(format!("BLPOP wakeup: client_a error: {}", e)),
        }
    }

    pub async fn test_wrongtype_get_on_list(client: &mut RedisClient) -> Result<(), String> {
        let _ = client.cmd(&["RPUSH", "test_rs:wt_list", "a"]).await?;
        let r = client.cmd(&["GET", "test_rs:wt_list"]).await?;
        match &r {
            RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
            _ => Err(format!("GET on list: expected WRONGTYPE, got {}", r)),
        }
    }

    pub async fn test_wrongtype_llen_on_string(client: &mut RedisClient) -> Result<(), String> {
        let _ = client.cmd(&["SET", "test_rs:wt_str", "val"]).await?;
        let r = client.cmd(&["LLEN", "test_rs:wt_str"]).await?;
        match &r {
            RespType::Error(msg) if msg.to_lowercase().contains("wrongtype") => Ok(()),
            _ => Err(format!("LLEN on string: expected WRONGTYPE, got {}", r)),
        }
    }
}

// ── Benchmark functions ────────────────────────────────────────────────

pub mod benchmarks {
    use crate::BenchResult;
    use crate::RedisClient;
    use tokio::time::Instant;

    pub async fn bench_set_get_throughput(client: &mut RedisClient, n: u64) -> Result<BenchResult, String> {
        let start = Instant::now();
        for i in 0..n {
            let key = format!("stress:k{}", i);
            let val = format!("v{}", i);
            let _ = client.cmd(&["SET", &key, &val]).await?;
            let _ = client.cmd(&["GET", &key]).await?;
        }
        let elapsed = start.elapsed();
        Ok(BenchResult {
            name: "SET+GET throughput",
            ops: n * 2,
            elapsed_ms: elapsed.as_millis() as u64,
            avg_latency_us: if n > 0 { elapsed.as_micros() as f64 / (n * 2) as f64 } else { 0.0 },
        })
    }

    pub async fn bench_large_value(client: &mut RedisClient, sizes: &[usize]) -> Result<Vec<BenchResult>, String> {
        let mut results = Vec::new();
        for &size in sizes {
            let value = "x".repeat(size);
            let key = "stress:large";
            let start = Instant::now();
            let _ = client.cmd(&["SET", key, &value]).await?;
            let set_elapsed = start.elapsed();
            let _ = client.cmd(&["GET", key]).await?;
            results.push(BenchResult {
                name: "large_value",
                ops: 1,
                elapsed_ms: set_elapsed.as_millis() as u64,
                avg_latency_us: set_elapsed.as_micros() as f64,
            });
        }
        Ok(results)
    }

    pub async fn bench_many_keys(client: &mut RedisClient, n: u64) -> Result<BenchResult, String> {
        let start = Instant::now();
        for i in 0..n {
            let _ = client.cmd(&["SET", &format!("stress:many:{i}"), &format!("v{i}")]).await?;
        }
        let set_elapsed = start.elapsed();
        let start = Instant::now();
        for i in 0..n {
            let _ = client.cmd(&["GET", &format!("stress:many:{i}")]).await?;
        }
        let get_elapsed = start.elapsed();
        let total_ms = (set_elapsed + get_elapsed).as_millis() as u64;
        Ok(BenchResult {
            name: "Many keys SET+GET",
            ops: n * 2,
            elapsed_ms: total_ms,
            avg_latency_us: (set_elapsed + get_elapsed).as_micros() as f64 / (n * 2) as f64,
        })
    }

    pub async fn bench_concurrent_connections(addr: &str, m: usize) -> Result<BenchResult, String> {
        let start = Instant::now();
        let mut handles = Vec::new();
        for cid in 0..m {
            let addr = addr.to_string();
            handles.push(tokio::spawn(async move {
                let mut client = RedisClient::connect(&addr).await?;
                let _ = client.cmd(&["SET", &format!("stress:conc:{cid}"), "val"]).await?;
                let _ = client.cmd(&["GET", &format!("stress:conc:{cid}")]).await?;
                Ok::<_, String>(())
            }));
        }
        let mut ok = 0;
        for h in handles {
            if let Ok(Ok(())) = h.await { ok += 1; }
        }
        let elapsed = start.elapsed();
        Ok(BenchResult {
            name: "concurrent_connections",
            ops: ok * 2,
            elapsed_ms: elapsed.as_millis() as u64,
            avg_latency_us: if ok > 0 { elapsed.as_micros() as f64 / (ok * 2) as f64 } else { 0.0 },
        })
    }

    pub async fn bench_list_massive_rpush_lrange(client: &mut RedisClient, n: u64) -> Result<BenchResult, String> {
        let key = "stress:biglist";
        let mut args: Vec<&str> = vec!["RPUSH", key];
        let num_strs: Vec<String> = (0..n).map(|i| i.to_string()).collect();
        let str_refs: Vec<&str> = num_strs.iter().map(|s| s.as_str()).collect();
        args.extend(&str_refs);
        let start = Instant::now();
        let _ = client.cmd(&args).await?;
        let rpush_elapsed = start.elapsed();
        let _ = client.cmd(&["LRANGE", key, "0", "-1"]).await?;
        Ok(BenchResult {
            name: "list_massive",
            ops: n,
            elapsed_ms: rpush_elapsed.as_millis() as u64,
            avg_latency_us: rpush_elapsed.as_micros() as f64 / n as f64,
        })
    }
}

// ── Metadata: test definitions ─────────────────────────────────────────

pub struct TestDef {
    pub name: &'static str,
    pub category: &'static str,
    pub category_filter: &'static str,
    pub stages: &'static str,
}

pub const ALL_TESTS: &[TestDef] = &[
    // Connection
    TestDef { name: "PING",                  category: "Connection", category_filter: "Connection", stages: "Stages 1-5" },
    TestDef { name: "ECHO simple",           category: "Connection", category_filter: "Connection", stages: "Stages 1-5" },
    TestDef { name: "ECHO with spaces",      category: "Connection", category_filter: "Connection", stages: "Stages 1-5" },
    TestDef { name: "Unknown command",       category: "Connection", category_filter: "Connection", stages: "Stages 1-5" },
    // String
    TestDef { name: "SET+GET roundtrip",     category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "GET nonexistent key",   category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET overwrite",         category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET with EX",           category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET with PX",           category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET wrong arg count",   category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET invalid flag",      category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET invalid expiry",    category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET empty value",       category: "String",     category_filter: "String",     stages: "Stage 6" },
    TestDef { name: "SET binary data",       category: "String",     category_filter: "String",     stages: "Stage 6" },
    // Expiry
    TestDef { name: "EX expiry",             category: "Expiry",     category_filter: "Expiry",     stages: "Stage 7" },
    TestDef { name: "PX expiry",             category: "Expiry",     category_filter: "Expiry",     stages: "Stage 7" },
    TestDef { name: "Background expiry",     category: "Expiry",     category_filter: "Expiry",     stages: "Stage 7" },
    // List
    TestDef { name: "RPUSH new key",         category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "RPUSH existing key",    category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LPUSH new key",         category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LRANGE positive",       category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LRANGE negative",       category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LRANGE out of bounds",  category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LRANGE empty key",      category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LLEN",                  category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LLEN empty key",        category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LPOP single",           category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LPOP with count",       category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LPOP count=0",          category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LPOP empty key",        category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LPOP count > len",      category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "LRANGE 1000 elements",  category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    TestDef { name: "List empty string",     category: "List",       category_filter: "List",       stages: "Stages 8-16" },
    // BLPOP
    TestDef { name: "BLPOP immediate",       category: "BLPOP",      category_filter: "BLPOP",      stages: "Stages 17-18" },
    TestDef { name: "BLPOP timeout",         category: "BLPOP",      category_filter: "BLPOP",      stages: "Stages 17-18" },
    TestDef { name: "BLPOP multi key",       category: "BLPOP",      category_filter: "BLPOP",      stages: "Stages 17-18" },
    TestDef { name: "BLPOP wakeup",          category: "BLPOP",      category_filter: "BLPOP",      stages: "Stages 17-18" },
    // WRONGTYPE
    TestDef { name: "GET on list",           category: "WRONGTYPE",  category_filter: "WRONGTYPE",  stages: "Edge Cases" },
    TestDef { name: "LLEN on string",        category: "WRONGTYPE",  category_filter: "WRONGTYPE",  stages: "Edge Cases" },
];

pub struct BenchmarkDef {
    pub name: &'static str,
    pub filter: &'static str,
    pub description: &'static str,
}

pub const BENCHMARKS: &[BenchmarkDef] = &[
    BenchmarkDef { name: "SET+GET throughput",     filter: "throughput",    description: "5000 iterations" },
    BenchmarkDef { name: "Large values",           filter: "large_value",   description: "1KB / 10KB / 100KB" },
    BenchmarkDef { name: "Many keys",              filter: "many_keys",     description: "1000 keys" },
    BenchmarkDef { name: "Concurrent connections", filter: "concurrent",    description: "10 connections" },
    BenchmarkDef { name: "Large list",             filter: "list",          description: "5000 elements" },
];

// ── Dispatch functions ─────────────────────────────────────────────────

pub async fn run_test(name: &str, client: &mut RedisClient) -> Result<(), String> {
    match name {
        "PING"              => functional_tests::test_ping(client).await,
        "ECHO simple"       => functional_tests::test_echo_simple(client).await,
        "ECHO with spaces"   => functional_tests::test_echo_spaces(client).await,
        "Unknown command"   => functional_tests::test_unknown_command(client).await,
        "SET+GET roundtrip" => functional_tests::test_set_get_roundtrip(client).await,
        "GET nonexistent key" => functional_tests::test_get_nonexistent(client).await,
        "SET overwrite"          => functional_tests::test_set_overwrite(client).await,
        "SET with EX"            => functional_tests::test_set_with_ex(client).await,
        "SET with PX"            => functional_tests::test_set_with_px(client).await,
        "SET wrong arg count"    => functional_tests::test_set_wrong_args(client).await,
        "SET invalid flag"       => functional_tests::test_set_invalid_flag(client).await,
        "SET invalid expiry"     => functional_tests::test_set_invalid_expiry(client).await,
        "SET empty value"        => functional_tests::test_set_empty_value(client).await,
        "SET binary data"        => functional_tests::test_set_binary_data(client).await,
        "EX expiry"              => functional_tests::test_ex_actual_expiry(client).await,
        "PX expiry"              => functional_tests::test_px_actual_expiry(client).await,
        "Background expiry"      => functional_tests::test_expiry_background_cleanup(client).await,
        "RPUSH new key"          => functional_tests::test_rpush_new_key(client).await,
        "RPUSH existing key"     => functional_tests::test_rpush_existing_key(client).await,
        "LPUSH new key"          => functional_tests::test_lpush_new_key(client).await,
        "LRANGE positive"        => functional_tests::test_lrange_positive_indices(client).await,
        "LRANGE negative"        => functional_tests::test_lrange_negative_indices(client).await,
        "LRANGE out of bounds"   => functional_tests::test_lrange_out_of_bounds(client).await,
        "LRANGE empty key"       => functional_tests::test_lrange_empty_key(client).await,
        "LLEN"                   => functional_tests::test_llen(client).await,
        "LLEN empty key"         => functional_tests::test_llen_empty_key(client).await,
        "LPOP single"            => functional_tests::test_lpop_single(client).await,
        "LPOP with count"        => functional_tests::test_lpop_with_count(client).await,
        "LPOP count=0"           => functional_tests::test_lpop_count_zero(client).await,
        "LPOP empty key"         => functional_tests::test_lpop_empty_key(client).await,
        "LPOP count > len"       => functional_tests::test_lpop_count_larger_than_list(client).await,
        "LRANGE 1000 elements"   => functional_tests::test_large_list_lrange(client).await,
        "List empty string"      => functional_tests::test_list_empty_string_element(client).await,
        "BLPOP immediate"        => functional_tests::test_blpop_immediate(client).await,
        "BLPOP timeout"          => functional_tests::test_blpop_timeout(client).await,
        "BLPOP multi key"        => functional_tests::test_blpop_multi_key(client).await,
        "BLPOP wakeup"           => functional_tests::test_blpop_wakeup(client).await,
        "GET on list"            => functional_tests::test_wrongtype_get_on_list(client).await,
        "LLEN on string"         => functional_tests::test_wrongtype_llen_on_string(client).await,
        _ => Err(format!("unknown test: {name}")),
    }
}

pub async fn run_bench(filter: &str, client: &mut RedisClient, addr: &str) -> Result<Vec<BenchResult>, String> {
    match filter {
        "throughput"  => benchmarks::bench_set_get_throughput(client, 5000).await.map(|r| vec![r]),
        "large_value" => benchmarks::bench_large_value(client, &[1024, 10240, 102400]).await,
        "many_keys"   => benchmarks::bench_many_keys(client, 1000).await.map(|r| vec![r]),
        "concurrent"  => benchmarks::bench_concurrent_connections(addr, 10).await.map(|r| vec![r]),
        "list"        => benchmarks::bench_list_massive_rpush_lrange(client, 5000).await.map(|r| vec![r]),
        _ => Err(format!("unknown benchmark: {filter}")),
    }
}
