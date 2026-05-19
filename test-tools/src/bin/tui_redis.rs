use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};
use std::sync::mpsc;
use std::time::Duration;
use test_tools::{ALL_TESTS, BENCHMARKS, run_test, run_bench, TestResult, BenchResult, RedisClient};
use tokio::runtime::{Builder as RtBuilder, Runtime};

// ── Category definitions (metadata only) ─────────────────────────────────

struct Category {
    name: &'static str,
    stages: &'static str,
    filters: &'static [&'static str],
}

const CATEGORIES: &[Category] = &[
    Category { name: "Base",       stages: "Base",    filters: &["Base"] },
    Category { name: "List",       stages: "List",    filters: &["List"] },
    Category { name: "Stream",     stages: "Stream",  filters: &["Stream"] },
    Category { name: "Hash",       stages: "Hash",    filters: &["Hash"] },
    Category { name: "Set",        stages: "Set",     filters: &["Set"] },
    Category { name: "ZSet",       stages: "ZSet",    filters: &["ZSet"] },
];

// ── Hierarchical select items ────────────────────────────────────────────

#[derive(Clone)]
enum FilterSpec {
    Category(&'static str),
    Subcategory(&'static str, &'static str),
}

/// Maps index into the visible flat list to actual item
#[derive(Clone, Copy)]
struct VisibleItem {
    cat_idx: usize,
    sub_idx: Option<usize>,  // None = category row, Some = subcategory row
}

/// Pre-computed subcategory info per category
struct SubcatInfo {
    labels: Vec<&'static str>,   // subcategory names, sorted, deduped
    counts: Vec<usize>,          // test count per subcategory
}

fn build_subcat_info() -> Vec<SubcatInfo> {
    let mut result = Vec::new();
    for cat in CATEGORIES {
        let mut subs: Vec<&str> = ALL_TESTS.iter()
            .filter(|t| t.category_filter == cat.filters[0] && t.subcategory.is_some())
            .map(|t| t.subcategory.unwrap())
            .collect();
        subs.sort();
        subs.dedup();
        let counts: Vec<usize> = subs.iter().map(|&sub|
            ALL_TESTS.iter()
                .filter(|t| t.category_filter == cat.filters[0] && t.subcategory == Some(sub))
                .count()
        ).collect();
        result.push(SubcatInfo { labels: subs, counts });
    }
    result
}

/// Build visible items for the current frame based on expanded state
fn visible_items(expanded: &[bool], subcat_info: &[SubcatInfo]) -> Vec<VisibleItem> {
    let mut v = Vec::new();
    for ci in 0..CATEGORIES.len() {
        v.push(VisibleItem { cat_idx: ci, sub_idx: None });
        if expanded[ci] {
            for si in 0..subcat_info[ci].labels.len() {
                v.push(VisibleItem { cat_idx: ci, sub_idx: Some(si) });
            }
        }
    }
    v
}

// ── Message types for background task communication ──────────────────────

enum UiMsg {
    Line(String),
    Done,
}

// ── Mode ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Functional,
    Stress,
}

// ── App state ────────────────────────────────────────────────────────────

enum Screen {
    Select,
    Running,
    Results,
}

struct App {
    screen: Screen,
    mode: Mode,
    list_state: ListState,

    // Selection state (one per CATEGORIES)
    cat_checked: Vec<bool>,
    expanded: Vec<bool>,
    sub_checked: Vec<Vec<bool>>,   // [cat_idx][sub_idx]

    // Pre-computed caches
    subcat_info: Vec<SubcatInfo>,

    // Stress mode
    stress_checked: Vec<bool>,

    // Output / results
    output_lines: Vec<String>,
    results: Vec<TestResult>,
    bench_results: Vec<BenchResult>,
    scroll: u16,
    exit: bool,
    rx: Option<mpsc::Receiver<UiMsg>>,
    test_join: Option<tokio::task::JoinHandle<()>>,
}

impl App {
    fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let subcat_info = build_subcat_info();
        let cat_checked = vec![true; CATEGORIES.len()];
        let sub_checked: Vec<Vec<bool>> = subcat_info.iter()
            .map(|info| vec![true; info.labels.len()])
            .collect();
        Self {
            screen: Screen::Select,
            mode: Mode::Functional,
            list_state,
            cat_checked,
            expanded: subcat_info.iter().map(|info| !info.labels.is_empty()).collect(),
            sub_checked,
            subcat_info,
            stress_checked: vec![true; BENCHMARKS.len()],
            output_lines: Vec::new(),
            results: Vec::new(),
            bench_results: Vec::new(),
            scroll: 0,
            exit: false,
            rx: None,
            test_join: None,
        }
    }

    fn visible(&self) -> Vec<VisibleItem> {
        visible_items(&self.expanded, &self.subcat_info)
    }

    fn visible_count(&self) -> usize {
        self.visible().len()
    }

    fn toggle_current(&mut self) {
        let vi = self.visible();
        let idx = match self.list_state.selected() {
            Some(i) if i < vi.len() => i,
            _ => return,
        };
        let item = vi[idx];
        match item.sub_idx {
            None => {
                if item.cat_idx < self.cat_checked.len() {
                    let new_val = !self.cat_checked[item.cat_idx];
                    self.cat_checked[item.cat_idx] = new_val;
                    for si in 0..self.subcat_info[item.cat_idx].labels.len() {
                        if si < self.sub_checked[item.cat_idx].len() {
                            self.sub_checked[item.cat_idx][si] = new_val;
                        }
                    }
                }
            }
            Some(si) => {
                if item.cat_idx < self.sub_checked.len() && si < self.sub_checked[item.cat_idx].len() {
                    self.sub_checked[item.cat_idx][si] = !self.sub_checked[item.cat_idx][si];
                    let all_checked = self.subcat_info[item.cat_idx].labels.iter().enumerate()
                        .all(|(i, _)| self.sub_checked[item.cat_idx][i]);
                    self.cat_checked[item.cat_idx] = all_checked;
                }
            }
        }
    }

    fn expand_current(&mut self) {
        let vi = self.visible();
        let idx = match self.list_state.selected() {
            Some(i) if i < vi.len() => i,
            _ => return,
        };
        let item = vi[idx];
        if item.sub_idx.is_none() && !self.subcat_info[item.cat_idx].labels.is_empty() {
            self.expanded[item.cat_idx] = true;
        }
    }

    fn collapse_current(&mut self) {
        let vi = self.visible();
        let idx = match self.list_state.selected() {
            Some(i) if i < vi.len() => i,
            _ => return,
        };
        let item = vi[idx];
        if item.sub_idx.is_none() {
            self.expanded[item.cat_idx] = false;
        }
    }

    fn select_all(&mut self, val: bool) {
        for c in &mut self.cat_checked {
            *c = val;
        }
        for row in &mut self.sub_checked {
            for c in row {
                *c = val;
            }
        }
    }

    fn selected_count(&self) -> usize {
        let mut total = 0;
        for (ci, cat) in CATEGORIES.iter().enumerate() {
            if self.cat_checked[ci] {
                total += ALL_TESTS.iter().filter(|t| t.category_filter == cat.filters[0]).count();
            } else {
                for (si, _) in self.subcat_info[ci].labels.iter().enumerate() {
                    if self.sub_checked[ci][si] {
                        total += ALL_TESTS.iter()
                            .filter(|t| t.category_filter == cat.filters[0]
                                && t.subcategory == Some(self.subcat_info[ci].labels[si]))
                            .count();
                    }
                }
            }
        }
        total
    }

    fn selected_filter_specs(&self) -> Vec<FilterSpec> {
        let mut specs = Vec::new();
        for (ci, cat) in CATEGORIES.iter().enumerate() {
            if self.cat_checked[ci] {
                specs.push(FilterSpec::Category(cat.filters[0]));
            } else {
                for (si, &sub) in self.subcat_info[ci].labels.iter().enumerate() {
                    if self.sub_checked[ci][si] {
                        specs.push(FilterSpec::Subcategory(cat.filters[0], sub));
                    }
                }
            }
        }
        specs
    }

    fn start_tests(&mut self, rt: &Runtime) {
        if let Some(h) = self.test_join.take() {
            h.abort();
        }

        self.output_lines.clear();
        self.results.clear();
        self.bench_results.clear();
        self.scroll = 0;

        let (tx, rx) = mpsc::channel::<UiMsg>();
        self.rx = Some(rx);

        let addr = "127.0.0.1:6379".to_string();

        match self.mode {
            Mode::Functional => {
                let filter_specs = self.selected_filter_specs();

                if filter_specs.is_empty() {
                    self.output_lines.push("[WARN] No items selected. Press B to go back.".to_string());
                    self.screen = Screen::Results;
                    return;
                }

                let tx_clone = tx.clone();
                self.test_join = Some(rt.spawn(async move {
                    let mut client = match RedisClient::connect(&addr).await {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = tx_clone.send(UiMsg::Line(format!("FAILED to connect: {e}")));
                            let _ = tx_clone.send(UiMsg::Done);
                            return;
                        }
                    };

                    let _ = tx_clone.send(UiMsg::Line("Connected.".to_string()));

                    if let Err(e) = client.cmd(&["FLUSHDB"]).await {
                        let _ = tx_clone.send(UiMsg::Line(format!("FLUSHDB failed: {e}")));
                    }

                    let mut current_cat = "";
                    let mut current_subcat = "";
                    let mut passed = 0u32;
                    let mut failed = 0u32;

                    for test in ALL_TESTS.iter().filter(|t| {
                        filter_specs.iter().any(|fs| match fs {
                            FilterSpec::Category(cat) => t.category_filter == *cat,
                            FilterSpec::Subcategory(cat, sub) => t.category_filter == *cat && t.subcategory == Some(sub),
                        })
                    }) {
                        if test.category != current_cat {
                            current_cat = test.category;
                            current_subcat = "";
                            let _ = tx_clone.send(UiMsg::Line(format!("\n[{}]", test.category)));
                        }
                        if test.subcategory.is_some() && test.subcategory != Some(current_subcat) {
                            let sc = test.subcategory.unwrap_or("");
                            let _ = tx_clone.send(UiMsg::Line(format!("  [{sc}]")));
                            current_subcat = sc;
                        }
                        let indent = if test.subcategory.is_some() { "    " } else { "  " };
                        match run_test(test, &mut client).await {
                            Ok(()) => {
                                let _ = tx_clone.send(UiMsg::Line(format!("{indent}[PASS] {}", test.name)));
                                passed += 1;
                            }
                            Err(e) => {
                                let _ = tx_clone.send(UiMsg::Line(format!("{indent}[FAIL] {}", test.name)));
                                let _ = tx_clone.send(UiMsg::Line(format!("           {e}")));
                                failed += 1;
                            }
                        }
                    }

                    let total = passed + failed;
                    let _ = tx_clone.send(UiMsg::Line(String::new()));
                    if failed == 0 {
                        let _ = tx_clone.send(UiMsg::Line(format!("=> All {passed} passed")));
                    } else {
                        let _ = tx_clone.send(UiMsg::Line(format!("=> {passed} passed, {failed} failed, {total} total")));
                    }

                    let _ = tx_clone.send(UiMsg::Done);
                }));
            }
            Mode::Stress => {
                let filters: Vec<String> = BENCHMARKS.iter().enumerate()
                    .filter(|(i, _)| self.stress_checked[*i])
                    .map(|(_, b)| b.filter.to_string())
                    .collect();

                if filters.is_empty() {
                    self.output_lines.push("[WARN] No benchmarks selected. Press B to go back.".to_string());
                    self.screen = Screen::Results;
                    return;
                }

                let tx_clone = tx.clone();
                self.test_join = Some(rt.spawn(async move {
                    let mut client = match RedisClient::connect(&addr).await {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = tx_clone.send(UiMsg::Line(format!("Failed to connect: {e}")));
                            let _ = tx_clone.send(UiMsg::Done);
                            return;
                        }
                    };

                    let _ = tx_clone.send(UiMsg::Line("Connected.".to_string()));

                    for filter in &filters {
                        if let Some(bench) = BENCHMARKS.iter().find(|b| b.filter == filter.as_str()) {
                            let _ = tx_clone.send(UiMsg::Line(format!("\n── {} ──", bench.name)));
                        }
                        match run_bench(filter, &mut client, &addr).await {
                            Ok(results) => {
                                for r in &results {
                                    let _ = tx_clone.send(UiMsg::Line(
                                        format!("  ops={}  time={}ms  qps={:.0}  avg_lat={:.1}µs",
                                            r.ops, r.elapsed_ms, r.qps(), r.avg_latency_us),
                                    ));
                                }
                            }
                            Err(e) => {
                                let _ = tx_clone.send(UiMsg::Line(format!("  Error: {e}")));
                            }
                        }
                    }

                    let _ = tx_clone.send(UiMsg::Done);
                }));
            }
        }

        self.screen = Screen::Running;
    }

    fn poll_test_output(&mut self) {
        let rx = match &self.rx {
            Some(r) => r,
            None => return,
        };

        let mut done = false;
        loop {
            match rx.try_recv() {
                Ok(UiMsg::Line(line)) => self.output_lines.push(line),
                Ok(UiMsg::Done) => {
                    done = true;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    done = true;
                    break;
                }
            }
        }

        if done {
            if let Some(h) = self.test_join.take() {
                h.abort();
            }
            self.rx = None;
            self.screen = Screen::Results;
        }
    }
}

// ── UI rendering ─────────────────────────────────────────────────────────

fn draw_select(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    // ── Title with mode tabs ──────────────────────────────────────────

    let (mode_func, mode_stress) = match app.mode {
        Mode::Functional => (
            Span::styled(
                " [功能测试]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  压力测试 ", Style::default().fg(Color::DarkGray)),
        ),
        Mode::Stress => (
            Span::styled("  功能测试 ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                " [压力测试]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ),
    };

    let title = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            " Redis Test Selector ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled(" ", Style::default()),
            mode_func,
            Span::styled("  ", Style::default()),
            mode_stress,
            Span::styled("   (Tab切换)", Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // ── Content ───────────────────────────────────────────────────────

    match app.mode {
        Mode::Functional => {
            let vi = app.visible();
            let items: Vec<ListItem> = vi.iter().map(|vitem| {
                let is_cat = vitem.sub_idx.is_none();
                let (checked, has_subcats) = if is_cat {
                    (app.cat_checked[vitem.cat_idx], !app.subcat_info[vitem.cat_idx].labels.is_empty())
                } else {
                    let si = vitem.sub_idx.unwrap();
                    (app.sub_checked[vitem.cat_idx][si], false)
                };
                let is_expanded = is_cat && app.expanded[vitem.cat_idx];
                let has_partial = is_cat && has_subcats && !checked
                    && app.sub_checked[vitem.cat_idx].iter().any(|&c| c);

                let check = if has_partial { "[-]" } else if checked { "[x]" } else { "[ ]" };
                let expand_mark = if has_subcats {
                    if is_expanded { "▼" } else { "▶" }
                } else {
                    " "
                };
                let prefix = if is_cat { "" } else { "  " };

                let color = if checked {
                    if is_cat { Color::Green } else { Color::Cyan }
                } else if has_partial {
                    Color::Yellow
                } else {
                    Color::Gray
                };

                let label = if is_cat {
                    CATEGORIES[vitem.cat_idx].name
                } else {
                    app.subcat_info[vitem.cat_idx].labels[vitem.sub_idx.unwrap()]
                };

                let mut spans = vec![
                    Span::styled(
                        format!(" {} {} {}{:<12}", check, expand_mark, prefix, label),
                        Style::default().fg(color),
                    ),
                ];
                if is_cat {
                    spans.push(Span::styled(
                        format!("  ({})", CATEGORIES[vitem.cat_idx].stages),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                if is_cat && !has_subcats {
                    let total = ALL_TESTS.iter().filter(|t| t.category_filter == CATEGORIES[vitem.cat_idx].filters[0]).count();
                    spans.push(Span::styled(
                        format!("  {} tests", total),
                        Style::default().fg(Color::DarkGray),
                    ));
                } else if is_cat && has_subcats {
                    let total = ALL_TESTS.iter().filter(|t| t.category_filter == CATEGORIES[vitem.cat_idx].filters[0]).count();
                    spans.push(Span::styled(
                        format!("  {} tests", total),
                        Style::default().fg(Color::DarkGray),
                    ));
                } else {
                    let si = vitem.sub_idx.unwrap();
                    spans.push(Span::styled(
                        format!("  {} tests", app.subcat_info[vitem.cat_idx].counts[si]),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                ListItem::new(Line::from(spans))
            }).collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_stateful_widget(list, chunks[1], &mut app.list_state);
        }
        Mode::Stress => {
            let items: Vec<ListItem> = BENCHMARKS
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let check = if app.stress_checked[i] { "[x]" } else { "[ ]" };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {}  {:<24}", check, item.name),
                            Style::default().fg(if app.stress_checked[i] {
                                Color::Green
                            } else {
                                Color::Gray
                            }),
                        ),
                        Span::styled(
                            format!("  {}", item.description),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_stateful_widget(list, chunks[1], &mut app.list_state);
        }
    }

    // ── Summary ───────────────────────────────────────────────────────

    let summary_text = match app.mode {
        Mode::Functional => {
            format!(" Total: {} tests selected", app.selected_count())
        }
        Mode::Stress => {
            let n = app.stress_checked.iter().filter(|&c| *c).count();
            format!(" Total: {n} / {} benchmarks selected", BENCHMARKS.len())
        }
    };

    let summary = Paragraph::new(Line::from(vec![Span::styled(
        summary_text,
        Style::default().fg(Color::Yellow),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(summary, chunks[2]);

    // ── Help ──────────────────────────────────────────────────────────

    let help = match app.mode {
        Mode::Functional => Paragraph::new(Line::from(vec![
            Span::styled(" \u{2191}\u{2193} Navig  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(":Tog  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(":Run  ", Style::default().fg(Color::DarkGray)),
            Span::styled("\u{2190}\u{2192}", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":Col/Exp  ", Style::default().fg(Color::DarkGray)),
            Span::styled("A", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":All  ", Style::default().fg(Color::DarkGray)),
            Span::styled("N", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":None  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(":Mode  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(":Quit", Style::default().fg(Color::DarkGray)),
        ])),
        Mode::Stress => Paragraph::new(Line::from(vec![
            Span::styled(" \u{2191}\u{2193} Navig  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Space", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(":Tog  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(":Run  ", Style::default().fg(Color::DarkGray)),
            Span::styled("A", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":All  ", Style::default().fg(Color::DarkGray)),
            Span::styled("N", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(":None  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(":Mode  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled(":Quit", Style::default().fg(Color::DarkGray)),
        ])),
    };

    f.render_widget(help, chunks[3]);
}

fn draw_running(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let title = Paragraph::new(Line::from(vec![Span::styled(
        " Running Tests...  (Q:Quit) ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let max_lines = (chunks[1].height as usize).saturating_sub(2);
    let start = if app.output_lines.len() > max_lines {
        app.output_lines.len() - max_lines
    } else {
        0
    };

    let output: Vec<ListItem> = app.output_lines[start..]
        .iter()
        .map(|l| {
            let style = if l.contains("[PASS]") {
                Style::default().fg(Color::Green)
            } else if l.contains("[FAIL]") {
                Style::default().fg(Color::Red)
            } else if l.contains("failed") || l.contains("error") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(l.clone(), style)))
        })
        .collect();

    let list = List::new(output).block(Block::default().borders(Borders::NONE));
    f.render_widget(list, chunks[1]);
}

fn draw_results(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![Span::styled(
        " Test Results ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let viewport = chunks[1].height as usize;
    let max_scroll = if app.output_lines.len() > viewport.saturating_sub(2) {
        app.output_lines.len() - viewport.saturating_sub(2)
    } else {
        0
    };
    if app.scroll as usize > max_scroll {
        app.scroll = max_scroll as u16;
    }

    let output: Vec<ListItem> = app.output_lines[app.scroll as usize..]
        .iter()
        .map(|l| {
            let style = if l.contains("[PASS]") {
                Style::default().fg(Color::Green)
            } else if l.contains("[FAIL]") {
                Style::default().fg(Color::Red)
            } else if l.starts_with("=>") {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(l.clone(), style)))
        })
        .collect();

    let list = List::new(output).block(Block::default().borders(Borders::NONE));
    f.render_widget(list, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            " \u{2191}\u{2193} Scroll  ",
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            "B",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Back  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "R",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Rerun  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Q",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Quit", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(help, chunks[2]);
}

// ── Main ─────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let rt = RtBuilder::new_multi_thread()
        .enable_io()
        .enable_time()
        .thread_stack_size(4 * 1024 * 1024)
        .build()
        .unwrap();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    let mut app = App::new();
    let tick_rate = Duration::from_millis(50);

    while !app.exit {
        app.poll_test_output();

        terminal.draw(|f| match app.screen {
            Screen::Select => draw_select(f, &mut app),
            Screen::Running => draw_running(f, &mut app),
            Screen::Results => draw_results(f, &mut app),
        })?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.screen {
                    Screen::Select => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if let Some(h) = app.test_join.take() {
                                h.abort();
                            }
                            app.exit = true;
                        }
                        KeyCode::Tab => {
                            app.mode = match app.mode {
                                Mode::Functional => Mode::Stress,
                                Mode::Stress => Mode::Functional,
                            };
                            app.list_state.select(Some(0));
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => app.select_all(true),
                        KeyCode::Char('n') | KeyCode::Char('N') => app.select_all(false),
                        KeyCode::Char(' ') => app.toggle_current(),
                        KeyCode::Enter => {
                            app.start_tests(&rt);
                        }
                        KeyCode::Right | KeyCode::Char('l') => app.expand_current(),
                        KeyCode::Left | KeyCode::Char('h') => app.collapse_current(),
                        KeyCode::Up | KeyCode::Char('k') => {
                            let i = app.list_state.selected().unwrap_or(0);
                            if i > 0 {
                                app.list_state.select(Some(i - 1));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let i = app.list_state.selected().unwrap_or(0);
                            if i + 1 < app.visible_count() {
                                app.list_state.select(Some(i + 1));
                            }
                        }
                        _ => {}
                    },
                    Screen::Results => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if let Some(h) = app.test_join.take() {
                                h.abort();
                            }
                            app.exit = true;
                        }
                        KeyCode::Char('b') | KeyCode::Char('B') => app.screen = Screen::Select,
                        KeyCode::Char('r') | KeyCode::Char('R') => app.start_tests(&rt),
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.scroll > 0 {
                                app.scroll -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.scroll += 1;
                        }
                        _ => {}
                    },
                    Screen::Running => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if let Some(h) = app.test_join.take() {
                                h.abort();
                            }
                            app.exit = true;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
