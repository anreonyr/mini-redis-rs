use test_tools::{ALL_TESTS, run_test, RedisClient, TestResult, TestDef};

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

#[tokio::main]
async fn main() {
    let filters: Vec<String> = std::env::args()
        .skip(1)
        .map(|a| a.to_uppercase())
        .collect();
    let has_filters = !filters.is_empty();

    let is_enabled = |def: &TestDef| -> bool {
        if !has_filters { return true; }
        filters.iter().any(|f| {
            let f = f.as_str();
            f == def.category_filter.to_uppercase()
            || def.subcategory.map_or(false, |s| s.to_uppercase() == f)
            || def.name.to_uppercase() == f
        })
    };

    println!("{BOLD}Redis Test Runner v0.2.0{RESET}");
    println!("Target: 127.0.0.1:6379");
    if has_filters {
        println!("Filters: {}", filters.join(", "));
    }
    println!("─────────────────────────────────────────────────");

    let mut client = match RedisClient::connect("127.0.0.1:6379").await {
        Ok(c) => {
            println!("Connected.\n");
            c
        }
        Err(e) => {
            eprintln!("{RED}FAILED to connect: {e}{RESET}");
            std::process::exit(1);
        }
    };

    match client.cmd(&["FLUSHDB"]).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{YELLOW}FLUSHDB failed (server may not support it): {e}{RESET}");
        }
    }

    let mut results: Vec<TestResult> = Vec::new();
    let mut current_category = "";
    let mut current_subcategory = "";

    for test in ALL_TESTS.iter().filter(|t| is_enabled(t)) {
        if test.category != current_category {
            println!("\n{BOLD}[{}]{RESET}", test.category);
            current_category = test.category;
            current_subcategory = "";
        }
        if test.subcategory.is_some() && test.subcategory != Some(current_subcategory) {
            let sc = test.subcategory.unwrap_or("");
            println!("  {BOLD}[{sc}]{RESET}");
            current_subcategory = sc;
        }
        let indent = if test.subcategory.is_some() { "    " } else { "  " };
        match run_test(test, &mut client).await {
            Ok(()) => {
                println!("{indent}{GREEN}[PASS]{RESET} {}", test.name);
                results.push(TestResult::pass(test.name, test.category));
            }
            Err(e) => {
                println!("{indent}{RED}[FAIL]{RESET} {}", test.name);
                println!("           {YELLOW}{DIM}{e}{RESET}");
                results.push(TestResult::fail(test.name, test.category, e));
            }
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();
    let total = results.len();

    println!();
    if failed == 0 {
        println!("{GREEN}{BOLD}Results: {passed} passed, {failed} failed, {total} total{RESET}");
    } else {
        println!("{RED}{BOLD}Results: {passed} passed, {failed} failed, {total} total{RESET}");
    }

    std::process::exit(if failed > 0 { 1 } else { 0 });
}
