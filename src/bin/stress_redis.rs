use codecrafters_redis::resp::{Decoder, DecodeError, RespType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;

const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

struct RedisClient {
    stream: TcpStream,
    read_buf: Vec<u8>,
    decoder: Decoder,
}

impl RedisClient {
    async fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("failed to connect: {}", e))?;
        Ok(Self {
            stream,
            read_buf: Vec::with_capacity(8192),
            decoder: Decoder::new(),
        })
    }

    async fn cmd(&mut self, args: &[&str]) -> Result<RespType, String> {
        let items: Vec<RespType> = args
            .iter()
            .map(|a| RespType::BulkString(Some(bytes::Bytes::copy_from_slice(a.as_bytes()))))
            .collect();
        let request = RespType::Array(Some(items));

        self.stream
            .write_all(&request.serialize())
            .await
            .map_err(|e| format!("write: {}", e))?;

        loop {
            let mut buf = [0u8; 8192];
            let n = self
                .stream
                .read(&mut buf)
                .await
                .map_err(|e| format!("read: {}", e))?;
            if n == 0 {
                return Err("connection closed".to_string());
            }
            self.read_buf.extend_from_slice(&buf[..n]);

            match self.decoder.decode(&self.read_buf) {
                Ok((frame, consumed)) => {
                    self.read_buf.drain(..consumed);
                    return Ok(frame);
                }
                Err(DecodeError::Incomplete) => {
                    if self.read_buf.len() > 1024 * 1024 {
                        self.read_buf.clear();
                        return Err("buffer exceeded 1MB".to_string());
                    }
                    continue;
                }
                Err(DecodeError::Invalid(e)) => {
                    self.read_buf.clear();
                    return Err(format!("decode: {}", e));
                }
            }
        }
    }
}

struct BenchResult {
    name: &'static str,
    ops: u64,
    elapsed_ms: u64,
    avg_latency_us: f64,
}

impl BenchResult {
    fn qps(&self) -> f64 {
        if self.elapsed_ms == 0 {
            return 0.0;
        }
        self.ops as f64 / (self.elapsed_ms as f64 / 1000.0)
    }
}

fn print_result(r: &BenchResult) {
    println!(
        "  {BOLD}{}{RESET}",
        r.name
    );
    println!(
        "    ops={}  time={}ms  qps={:.0}  avg_lat={:.1}µs",
        r.ops, r.elapsed_ms, r.qps(), r.avg_latency_us,
    );
}

// ── Benchmarks ─────────────────────────────────────────────────────────────

async fn bench_set_get_throughput(client: &mut RedisClient, n: u64) -> Result<BenchResult, String> {
    let start = Instant::now();
    for i in 0..n {
        let key = format!("stress:k{}", i);
        let val = format!("v{}", i);
        let _ = client.cmd(&["SET", &key, &val]).await?;
        let _ = client.cmd(&["GET", &key]).await?;
    }
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis() as u64;
    Ok(BenchResult {
        name: "SET+GET throughput",
        ops: n * 2, // each iteration does SET + GET = 2 ops
        elapsed_ms,
        avg_latency_us: if n > 0 {
            elapsed.as_micros() as f64 / (n * 2) as f64
        } else {
            0.0
        },
    })
}

async fn bench_large_value(
    client: &mut RedisClient,
    sizes: &[usize],
) -> Result<Vec<BenchResult>, String> {
    let mut results = Vec::new();
    for &size in sizes {
        let value = "x".repeat(size);
        let key = "stress:large";

        let start = Instant::now();
        let _ = client.cmd(&["SET", key, &value]).await?;
        let set_elapsed = start.elapsed();

        let start = Instant::now();
        let r = client.cmd(&["GET", key]).await?;
        let get_elapsed = start.elapsed();

        let ok = match r {
            RespType::BulkString(Some(data)) => data.len() == size,
            _ => false,
        };

        results.push(BenchResult {
            name: "large_value",
            ops: 1,
            elapsed_ms: set_elapsed.as_millis() as u64,
            avg_latency_us: set_elapsed.as_micros() as f64,
        });
        println!(
            "  {BOLD}Large value {size}B{RESET}: SET={:.1}µs GET={:.1}µs ok={}",
            set_elapsed.as_micros() as f64,
            get_elapsed.as_micros() as f64,
            ok,
        );
    }
    Ok(results)
}

async fn bench_many_keys(client: &mut RedisClient, n: u64) -> Result<BenchResult, String> {
    let start = Instant::now();
    for i in 0..n {
        let key = format!("stress:many:{}", i);
        let val = format!("v{}", i);
        let _ = client.cmd(&["SET", &key, &val]).await?;
    }
    let set_elapsed = start.elapsed();

    let start = Instant::now();
    for i in 0..n {
        let key = format!("stress:many:{}", i);
        let _ = client.cmd(&["GET", &key]).await?;
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

async fn bench_concurrent_connections(addr: &str, m: usize) -> Result<BenchResult, String> {
    let start = Instant::now();
    let mut handles = Vec::new();

    for cid in 0..m {
        let addr = addr.to_string();
        handles.push(tokio::spawn(async move {
            let mut client = RedisClient::connect(&addr).await?;
            let key = format!("stress:conc:{}", cid);
            let _ = client.cmd(&["SET", &key, "val"]).await?;
            let _ = client.cmd(&["GET", &key]).await?;
            Ok::<_, String>(())
        }));
    }

    let mut ok = 0;
    for h in handles {
        match h.await {
            Ok(Ok(())) => ok += 1,
            _ => {}
        }
    }

    let elapsed = start.elapsed();
    Ok(BenchResult {
        name: "concurrent_connections",
        ops: ok * 2,
        elapsed_ms: elapsed.as_millis() as u64,
        avg_latency_us: if ok > 0 {
            elapsed.as_micros() as f64 / (ok * 2) as f64
        } else {
            0.0
        },
    })
}

async fn bench_list_massive_rpush_lrange(
    client: &mut RedisClient,
    n: u64,
) -> Result<BenchResult, String> {
    let key = "stress:biglist";
    let mut args: Vec<&str> = vec!["RPUSH", key];
    let num_strs: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    let str_refs: Vec<&str> = num_strs.iter().map(|s| s.as_str()).collect();
    args.extend(&str_refs);

    let start = Instant::now();
    let _ = client.cmd(&args).await?;
    let rpush_elapsed = start.elapsed();

    let start = Instant::now();
    let r = client.cmd(&["LRANGE", key, "0", "-1"]).await?;
    let lrange_elapsed = start.elapsed();

    let ok = match r {
        RespType::Array(Some(items)) => items.len() == n as usize,
        _ => false,
    };

    println!(
        "  {BOLD}List {n} elements{RESET}: RPUSH={:.1}ms LRANGE={:.1}ms ok={}",
        rpush_elapsed.as_millis() as f64,
        lrange_elapsed.as_millis() as f64,
        ok,
    );

    Ok(BenchResult {
        name: "list_massive",
        ops: n,
        elapsed_ms: rpush_elapsed.as_millis() as u64,
        avg_latency_us: rpush_elapsed.as_micros() as f64 / n as f64,
    })
}

// ── Main ───────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("{BOLD}{CYAN}Redis Stress Test{RESET}");
    println!("Target: 127.0.0.1:6379\n");

    let addr = "127.0.0.1:6379";

    // 1. Connection check
    let mut client = match RedisClient::connect(addr).await {
        Ok(c) => {
            println!("{GREEN}Connected.{RESET}\n");
            c
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            std::process::exit(1);
        }
    };

    // Ping check
    match client.cmd(&["PING"]).await {
        Ok(RespType::SimpleString(s)) if s == "PONG" => {
            println!("PING OK\n");
        }
        other => {
            eprintln!("PING failed: {:?}", other);
            std::process::exit(1);
        }
    }

    println!("{BOLD}── Benchmarks ──{RESET}\n");

    // 1. SET+GET throughput (10K ops)
    match bench_set_get_throughput(&mut client, 5000).await {
        Ok(r) => print_result(&r),
        Err(e) => eprintln!("  {YELLOW}SET/GET bench failed: {}{RESET}", e),
    }

    // 2. Large values
    println!();
    let _ = bench_large_value(&mut client, &[1024, 10240, 102400]).await;

    // 3. Many keys
    println!();
    match bench_many_keys(&mut client, 1000).await {
        Ok(r) => print_result(&r),
        Err(e) => eprintln!("  {YELLOW}many keys bench failed: {}{RESET}", e),
    }

    // 4. Concurrent connections
    println!();
    match bench_concurrent_connections(addr, 10).await {
        Ok(r) => print_result(&r),
        Err(e) => eprintln!("  {YELLOW}concurrent bench failed: {}{RESET}", e),
    }

    // 5. Large list
    println!();
    match bench_list_massive_rpush_lrange(&mut client, 5000).await {
        Ok(r) => print_result(&r),
        Err(e) => eprintln!("  {YELLOW}list bench failed: {}{RESET}", e),
    }

    println!("\n{BOLD}Done.{RESET}");
}
