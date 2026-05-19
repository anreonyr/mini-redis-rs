use test_tools::{BENCHMARKS, run_bench, RedisClient};
use mini_redis::protocol::resp::RespType;

const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

#[tokio::main]
async fn main() {
    let filters: Vec<String> = std::env::args().skip(1).map(|a| a.to_lowercase()).collect();
    let run = |name: &str| -> bool {
        filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str()))
    };

    println!("{BOLD}{CYAN}Redis Stress Test{RESET}");
    println!("Target: 127.0.0.1:6379\n");

    if !filters.is_empty() {
        println!("Filters: {}\n", filters.join(", "));
    }

    let addr = "127.0.0.1:6379";

    let mut client = match RedisClient::connect(addr).await {
        Ok(c) => {
            println!("{GREEN}Connected.{RESET}\n");
            c
        }
        Err(e) => {
            eprintln!("Failed to connect: {e}");
            std::process::exit(1);
        }
    };

    match client.cmd(&["PING"]).await {
        Ok(RespType::SimpleString(s)) if s == "PONG" => {
            println!("PING OK\n");
        }
        other => {
            eprintln!("PING failed: {other:?}");
            std::process::exit(1);
        }
    }

    println!("{BOLD}── Benchmarks ──{RESET}\n");

    for bench in BENCHMARKS.iter().filter(|b| run(b.filter)) {
        println!("{BOLD}{}{RESET}", bench.name);
        match run_bench(bench.filter, &mut client, addr).await {
            Ok(results) => {
                for r in &results {
                    println!(
                        "    ops={}  time={}ms  qps={:.0}  avg_lat={:.1}µs",
                        r.ops, r.elapsed_ms, r.qps(), r.avg_latency_us,
                    );
                }
            }
            Err(e) => eprintln!("  {YELLOW}Error: {e}{RESET}"),
        }
        println!();
    }

    println!("{BOLD}Done.{RESET}");
}
