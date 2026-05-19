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

// ── Test modules (split by category) ──────────────────────────────────

pub mod tests;

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

// ── tree_tests! macro: generates ALL_TESTS + run_test from tree ────────

macro_rules! tree_tests {
    (
        $(
            ($cat:expr, $filter:expr) [
                $(
                    ($sub:expr, $stages:expr) [
                        $($name:expr => $handler:path),+ $(,)?
                    ]
                ),* $(,)?
            ]
        ),+ $(,)?
    ) => {
        pub const ALL_TESTS: &[TestDef] = &[
            $(
                $(
                    $(
                        TestDef {
                            name: $name,
                            category: $cat,
                            subcategory: { let s = $sub; if s.is_empty() { None } else { Some(s) } },
                            category_filter: $filter,
                            stages: $stages,
                        },
                    )+
                )*
            )+
        ];

        pub fn run_test<'a>(def: &'a TestDef, client: &'a mut RedisClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
            // Wrap in async move so the giant match's state machine lives on the
            // heap (Box) instead of the stack, avoiding debug-mode stack overflow.
            Box::pin(async move {
                let subcat = def.subcategory.unwrap_or("");
                let name = def.name;
                match (subcat, name) {
                    $(
                        $(
                            $(
                                ($sub, $name) => $handler(client).await,
                            )+
                        )*
                    )+
                    _ => Err(format!("unknown test: {} / {}", subcat, name)),
                }
            })
        }
    };
}

// ── Metadata: test definitions ─────────────────────────────────────────

pub struct TestDef {
    pub name: &'static str,
    pub category: &'static str,
    pub subcategory: Option<&'static str>,
    pub category_filter: &'static str,
    pub stages: &'static str,
}

tree_tests! {
    ("Base", "Base") [
        ("CONNECTION", "Stages 1-5") [
            "PING"                => tests::connection::test_ping,
            "ECHO simple"         => tests::connection::test_echo_simple,
            "ECHO with spaces"    => tests::connection::test_echo_spaces,
            "Unknown command"     => tests::connection::test_unknown_command,
        ],
        ("STRING", "Stage 6") [
            "SET+GET roundtrip"   => tests::string::test_set_get_roundtrip,
            "GET nonexistent key" => tests::string::test_get_nonexistent,
            "SET overwrite"       => tests::string::test_set_overwrite,
            "SET with EX"         => tests::string::test_set_with_ex,
            "SET with PX"         => tests::string::test_set_with_px,
            "SET wrong arg count" => tests::string::test_set_wrong_args,
            "SET invalid flag"    => tests::string::test_set_invalid_flag,
            "SET invalid expiry"  => tests::string::test_set_invalid_expiry,
            "SET empty value"     => tests::string::test_set_empty_value,
            "SET binary data"     => tests::string::test_set_binary_data,
        ],
        ("STRING_EXT", "String") [
            "INCR new key"        => tests::string::test_incr_new_key,
            "INCR existing"       => tests::string::test_incr_existing,
            "DECR"                => tests::string::test_decr,
            "INCRBY"              => tests::string::test_incrby,
            "DECRBY"              => tests::string::test_decrby,
            "INCR wrong type"     => tests::string::test_incr_wrong_type,
            "INCR invalid value"  => tests::string::test_incr_invalid_value,
            "APPEND"              => tests::string::test_append,
            "APPEND new key"      => tests::string::test_append_new_key,
            "STRLEN"              => tests::string::test_strlen,
            "STRLEN nonexistent"  => tests::string::test_strlen_nonexistent,
            "MGET"                => tests::string::test_mget,
            "MSET"                => tests::string::test_mset,
        ],
        ("STRING_MORE", "String") [
            "GETSET"              => tests::string::test_getset,
            "GETRANGE"            => tests::string::test_getrange,
            "SETRANGE"            => tests::string::test_setrange,
            "MSETNX"              => tests::string::test_msetnx,
        ],
        ("EXPIRE", "Stage 7") [
            "EX expiry"           => tests::expiry::test_ex_actual_expiry,
            "PX expiry"           => tests::expiry::test_px_actual_expiry,
            "Background expiry"   => tests::expiry::test_expiry_background_cleanup,
            "EXPIRE basic"        => tests::expiry::test_expire_basic,
            "EXPIRE nonexistent"  => tests::expiry::test_expire_nonexistent,
            "TTL with expiry"     => tests::expiry::test_ttl_with_expiry,
            "TTL no expiry"       => tests::expiry::test_ttl_no_expiry,
            "TTL nonexistent"     => tests::expiry::test_ttl_nonexistent,
            "PERSIST basic"       => tests::expiry::test_persist_basic,
            "PERSIST nonexistent" => tests::expiry::test_persist_nonexistent,
            "PERSIST no expiry"   => tests::expiry::test_persist_no_expiry,
        ],
        ("WRONGTYPE", "Edge Cases") [
            "GET on list"         => tests::wrongtype::test_wrongtype_get_on_list,
            "LLEN on string"      => tests::wrongtype::test_wrongtype_llen_on_string,
            "RPUSH on string"     => tests::wrongtype::test_wrongtype_rpush_on_string,
            "LPOP on string"      => tests::wrongtype::test_wrongtype_lpop_on_string,
            "LRANGE on string"    => tests::wrongtype::test_wrongtype_lrange_on_string,
            "BLPOP on string"     => tests::wrongtype::test_wrongtype_blpop_on_string,
        ],
        ("COMMAND", "Registry") [
            "COMMAND"                 => tests::command::test_command_plain,
            "COMMAND INFO"            => tests::command::test_command_info_all,
            "COMMAND INFO specific"   => tests::command::test_command_info_specific,
            "COMMAND INFO nonexistent" => tests::command::test_command_info_nonexistent,
            "COMMAND unknown subcommand" => tests::command::test_command_unknown_subcommand,
        ],
        ("SERVER", "Server") [
            "FLUSHDB"             => tests::server::test_flushdb,
            "INFO"                => tests::server::test_info,
            "CONFIG GET dir"      => tests::server::test_config_get_dir,
            "CONFIG GET unknown"  => tests::server::test_config_get_unknown,
        ],
    ],
    ("Key", "Key") [
        ("DEL", "New") [
            "DEL single"           => tests::key::test_del_single,
            "DEL multiple"         => tests::key::test_del_multiple,
            "DEL nonexistent"      => tests::key::test_del_nonexistent,
        ],
        ("EXISTS", "New") [
            "EXISTS single"        => tests::key::test_exists_single,
            "EXISTS multiple"      => tests::key::test_exists_multiple,
            "EXISTS nonexistent"   => tests::key::test_exists_nonexistent,
        ],
        ("TYPE", "New") [
            "TYPE string"          => tests::key::test_type_string,
            "TYPE list"            => tests::key::test_type_list,
            "TYPE none"            => tests::key::test_type_none,
        ],
        ("KEYS", "New") [
            "KEYS pattern"         => tests::key::test_keys_pattern,
            "KEYS nomatch"         => tests::key::test_keys_nomatch,
        ],
        ("DBSIZE", "New") [
            "DBSIZE basic"         => tests::key::test_dbsize,
        ],
        ("RENAME", "New") [
            "RENAME basic"         => tests::key::test_rename,
        ],
        ("RENAMENX", "New") [
            "RENAMENX"             => tests::key::test_renamenx,
        ],
        ("RANDOMKEY", "New") [
            "RANDOMKEY"            => tests::key::test_randomkey,
        ],
    ],
    ("List", "List") [
        ("RPUSH", "Stages 8-16") [
            "RPUSH new key"       => tests::list::test_rpush_new_key,
            "RPUSH existing key"  => tests::list::test_rpush_existing_key,
            "List empty string"   => tests::list::test_list_empty_string_element,
        ],
        ("LPUSH", "Stages 8-16") [
            "LPUSH new key"       => tests::list::test_lpush_new_key,
        ],
        ("LRANGE", "Stages 8-16") [
            "LRANGE positive"     => tests::list::test_lrange_positive_indices,
            "LRANGE negative"     => tests::list::test_lrange_negative_indices,
            "LRANGE out of bounds" => tests::list::test_lrange_out_of_bounds,
            "LRANGE empty key"    => tests::list::test_lrange_empty_key,
            "LRANGE 1000 elements" => tests::list::test_large_list_lrange,
        ],
        ("LLEN", "Stages 8-16") [
            "LLEN"                => tests::list::test_llen,
            "LLEN empty key"      => tests::list::test_llen_empty_key,
        ],
        ("LPOP", "Stages 8-16") [
            "LPOP single"         => tests::list::test_lpop_single,
            "LPOP with count"     => tests::list::test_lpop_with_count,
            "LPOP count=0"        => tests::list::test_lpop_count_zero,
            "LPOP empty key"      => tests::list::test_lpop_empty_key,
            "LPOP count > len"    => tests::list::test_lpop_count_larger_than_list,
        ],
        ("RPOP", "New") [
            "RPOP single"         => tests::list::test_rpop_single,
            "RPOP with count"     => tests::list::test_rpop_with_count,
            "RPOP empty key"      => tests::list::test_rpop_empty_key,
        ],
        ("LINDEX", "New") [
            "LINDEX basic"        => tests::list::test_lindex_basic,
            "LINDEX out of bounds" => tests::list::test_lindex_out_of_bounds,
            "LINDEX nonexistent"  => tests::list::test_lindex_nonexistent,
        ],
        ("LREM", "New") [
            "LREM positive count" => tests::list::test_lrem_positive_count,
            "LREM negative count" => tests::list::test_lrem_negative_count,
            "LREM all"            => tests::list::test_lrem_all,
            "LREM nonexistent"    => tests::list::test_lrem_nonexistent,
        ],
        ("LTRIM", "New") [
            "LTRIM basic"         => tests::list::test_ltrim_basic,
            "LTRIM negative"      => tests::list::test_ltrim_negative_indices,
            "LTRIM nonexistent"   => tests::list::test_ltrim_nonexistent,
        ],
        ("RPOPLPUSH", "New") [
            "RPOPLPUSH"           => tests::list::test_rpoplpush,
        ],
        ("LSET", "New") [
            "LSET"                => tests::list::test_lset,
        ],
        ("BLPOP", "Stages 17-18") [
            "BLPOP immediate"       => tests::blpop::test_blpop_immediate,
            "BLPOP timeout"         => tests::blpop::test_blpop_timeout,
            "BLPOP multi key"       => tests::blpop::test_blpop_multi_key,
            "BLPOP wakeup"          => tests::blpop::test_blpop_wakeup,
            "BLPOP zero timeout with data" => tests::blpop::test_blpop_zero_timeout_with_data,
        ],
    ],
    ("Stream", "Stream") [
        ("XADD", "Stream") [
            "basic"               => tests::stream::test_xadd_basic,
            "explicit ID"         => tests::stream::test_xadd_explicit_id,
            "sequence auto"       => tests::stream::test_xadd_sequence_auto,
            "multiple"            => tests::stream::test_xadd_multiple,
        ],
        ("XLEN", "Stream") [
            "XLEN"                => tests::stream::test_xlen,
        ],
        ("XRANGE", "Stream") [
            "full"                => tests::stream::test_xrange_full,
            "range"               => tests::stream::test_xrange_range,
            "count"               => tests::stream::test_xrange_count,
        ],
        ("XREVRANGE", "Stream") [
            "XREVRANGE"           => tests::stream::test_xrevrange,
        ],
        ("XTRIM", "Stream") [
            "XTRIM"               => tests::stream::test_xtrim,
        ],
        ("XDEL", "Stream") [
            "XDEL"                => tests::stream::test_xdel,
        ],
        ("XREAD", "Stream") [
            "basic"               => tests::stream::test_xread_basic,
            "count"               => tests::stream::test_xread_count,
            "multi key"           => tests::stream::test_xread_multi_key,
        ],
        ("", "Stream") [
            "WRONGTYPE XADD on string" => tests::stream::test_wrongtype_xadd_on_string,
        ],
    ],
    ("Hash", "Hash") [
        ("HSET", "New") [
            "HSET new key"          => tests::hash::test_hset_new_key,
            "HSET multiple fields"  => tests::hash::test_hset_multiple_fields,
            "HSET overwrite"        => tests::hash::test_hset_overwrite,
        ],
        ("HGET", "New") [
            "HGET existing"         => tests::hash::test_hget_existing,
            "HGET nonexistent"      => tests::hash::test_hget_nonexistent,
            "HGET nonexistent key"  => tests::hash::test_hget_nonexistent_key,
        ],
        ("HDEL", "New") [
            "HDEL single"           => tests::hash::test_hdel_single,
            "HDEL multiple"         => tests::hash::test_hdel_multiple,
            "HDEL nonexistent"      => tests::hash::test_hdel_nonexistent,
        ],
        ("HGETALL", "New") [
            "HGETALL full"          => tests::hash::test_hgetall_full,
            "HGETALL empty"         => tests::hash::test_hgetall_empty,
        ],
        ("HEXISTS", "New") [
            "HEXISTS true"          => tests::hash::test_hexists_true,
            "HEXISTS false"         => tests::hash::test_hexists_false,
        ],
        ("HLEN", "New") [
            "HLEN basic"            => tests::hash::test_hlen,
            "HLEN empty"            => tests::hash::test_hlen_empty,
        ],
        ("HKEYS", "New") [
            "HKEYS basic"           => tests::hash::test_hkeys,
        ],
        ("HVALS", "New") [
            "HVALS basic"           => tests::hash::test_hvals,
        ],
        ("HINCRBY", "New") [
            "HINCRBY existing"      => tests::hash::test_hincrby,
            "HINCRBY new"           => tests::hash::test_hincrby_new,
        ],
        ("HINCRBYFLOAT", "New") [
            "HINCRBYFLOAT"          => tests::hash::test_hincrbyfloat,
        ],
        ("HSETNX", "New") [
            "HSETNX new"            => tests::hash::test_hsetnx,
        ],
        ("WRONGTYPE", "New") [
            "HGET on string"        => tests::wrongtype::test_wrongtype_hget_on_string,
            "HSET on string"        => tests::wrongtype::test_wrongtype_hset_on_string,
        ],
    ],
    ("Set", "Set") [
        ("SADD", "New") [
            "SADD new key"          => tests::set::test_sadd_new_key,
            "SADD existing"         => tests::set::test_sadd_existing_members,
            "SADD duplicate"        => tests::set::test_sadd_duplicate,
        ],
        ("SMEMBERS", "New") [
            "SMEMBERS basic"        => tests::set::test_smembers,
            "SMEMBERS empty"        => tests::set::test_smembers_empty_key,
        ],
        ("SISMEMBER", "New") [
            "SISMEMBER true"        => tests::set::test_sismember_true,
            "SISMEMBER false"       => tests::set::test_sismember_false,
            "SISMEMBER nonexistent" => tests::set::test_sismember_nonexistent_key,
        ],
        ("SREM", "New") [
            "SREM single"           => tests::set::test_srem_single,
            "SREM multiple"         => tests::set::test_srem_multiple,
            "SREM nonexistent"      => tests::set::test_srem_nonexistent,
        ],
        ("SCARD", "New") [
            "SCARD basic"           => tests::set::test_scard,
            "SCARD empty"           => tests::set::test_scard_empty,
        ],
        ("SPOP", "New") [
            "SPOP single"           => tests::set::test_spop_single,
            "SPOP count"            => tests::set::test_spop_count,
            "SPOP empty"            => tests::set::test_spop_empty,
        ],
        ("SRANDMEMBER", "New") [
            "SRANDMEMBER basic"     => tests::set::test_srandmember_basic,
        ],
        ("SUNION", "New") [
            "SUNION"                => tests::set::test_sunion,
        ],
        ("SINTER", "New") [
            "SINTER"                => tests::set::test_sinter,
        ],
        ("SDIFF", "New") [
            "SDIFF"                 => tests::set::test_sdiff,
        ],
        ("SMOVE", "New") [
            "SMOVE"                 => tests::set::test_smove,
        ],
        ("WRONGTYPE", "New") [
            "SADD on string"        => tests::wrongtype::test_wrongtype_sadd_on_string,
        ],
    ],
    ("ZSet", "ZSet") [
        ("ZADD", "New") [
            "ZADD new key"              => tests::zset::test_zadd_new_key,
            "ZADD update score"         => tests::zset::test_zadd_update_score,
            "ZADD existing and new"     => tests::zset::test_zadd_existing_and_new,
        ],
        ("ZRANGE", "New") [
            "ZRANGE full"               => tests::zset::test_zrange_by_index,
            "ZRANGE partial"            => tests::zset::test_zrange_partial,
            "ZRANGE withscores"         => tests::zset::test_zrange_withscores,
            "ZRANGE empty key"          => tests::zset::test_zrange_empty_key,
        ],
        ("ZRANK", "New") [
            "ZRANK existing"            => tests::zset::test_zrank_existing,
            "ZRANK nonexistent"         => tests::zset::test_zrank_nonexistent,
        ],
        ("ZSCORE", "New") [
            "ZSCORE existing"           => tests::zset::test_zscore_existing,
            "ZSCORE nonexistent"        => tests::zset::test_zscore_nonexistent,
        ],
        ("ZREM", "New") [
            "ZREM basic"                => tests::zset::test_zrem_basic,
            "ZREM nonexistent"          => tests::zset::test_zrem_nonexistent,
        ],
        ("ZCARD", "New") [
            "ZCARD basic"               => tests::zset::test_zcard,
            "ZCARD empty"               => tests::zset::test_zcard_empty,
        ],
        ("ZCOUNT", "New") [
            "ZCOUNT range"              => tests::zset::test_zcount,
            "ZCOUNT inf"                => tests::zset::test_zcount_inf,
        ],
        ("ZRANGEBYSCORE", "New") [
            "ZRANGEBYSCORE"             => tests::zset::test_zrangebyscore,
            "ZRANGEBYSCORE WITHSCORES"  => tests::zset::test_zrangebyscore_withscores,
        ],
        ("ZINCRBY", "New") [
            "ZINCRBY existing"          => tests::zset::test_zincrby,
            "ZINCRBY new"               => tests::zset::test_zincrby_new,
        ],
        ("ZREVRANGE", "New") [
            "ZREVRANGE full"            => tests::zset::test_zrevrange,
        ],
        ("ZREVRANK", "New") [
            "ZREVRANK"                  => tests::zset::test_zrevrank,
        ],
        ("ZREMRANGEBYRANK", "New") [
            "ZREMRANGEBYRANK"           => tests::zset::test_zremrangebyrank,
        ],
        ("ZREMRANGEBYSCORE", "New") [
            "ZREMRANGEBYSCORE"          => tests::zset::test_zremrangebyscore,
        ],
        ("ZREVRANGEBYSCORE", "New") [
            "ZREVRANGEBYSCORE"          => tests::zset::test_zrevrangebyscore,
        ],
        ("WRONGTYPE", "New") [
            "ZADD on string"            => tests::wrongtype::test_wrongtype_zadd_on_string,
        ],
    ],
}

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
