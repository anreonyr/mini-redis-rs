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
use tokio::runtime::Runtime;

// ── Category definitions ────────────────────────────────────────────────────

struct Category {
    name: &'static str,
    stages: &'static str,
    filters: &'static [&'static str],
}

const CATEGORIES: &[Category] = &[
    Category { name: "Connection", stages: "Stages 1-5",  filters: &["Connection"] },
    Category { name: "String",     stages: "Stage 6",      filters: &["String"] },
    Category { name: "Expiry",     stages: "Stage 7",      filters: &["Expiry"] },
    Category { name: "List",       stages: "Stages 8-16",  filters: &["List"] },
    Category { name: "BLPOP",      stages: "Stages 17-18", filters: &["BLPOP"] },
    Category { name: "WRONGTYPE",  stages: "Edge Cases",   filters: &["WRONGTYPE"] },
];

// ── Message types for background task communication ────────────────────────

enum UiMsg {
    Line(String),
    Done,
}

// ── Mode ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Functional,
    Stress,
}

// ── App state ───────────────────────────────────────────────────────────────

enum Screen {
    Select,
    Running,
    Results,
}

struct App {
    screen: Screen,
    mode: Mode,
    list_state: ListState,
    checked: Vec<bool>,
    stress_checked: Vec<bool>,
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
        Self {
            screen: Screen::Select,
            mode: Mode::Functional,
            list_state,
            checked: vec![true; CATEGORIES.len()],
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

    fn toggle_current(&mut self) {
        if let Some(i) = self.list_state.selected() {
            match self.mode {
                Mode::Functional => {
                    if i < self.checked.len() {
                        self.checked[i] = !self.checked[i];
                    }
                }
                Mode::Stress => {
                    if i < self.stress_checked.len() {
                        self.stress_checked[i] = !self.stress_checked[i];
                    }
                }
            }
        }
    }

    fn select_all(&mut self, val: bool) {
        match self.mode {
            Mode::Functional => {
                for c in &mut self.checked {
                    *c = val;
                }
            }
            Mode::Stress => {
                for c in &mut self.stress_checked {
                    *c = val;
                }
            }
        }
    }

    fn selected_count(&self) -> usize {
        match self.mode {
            Mode::Functional => {
                let selected_filters: Vec<&str> = CATEGORIES.iter().enumerate()
                    .filter(|(i, _)| self.checked[*i])
                    .flat_map(|(_, c)| c.filters.iter().copied())
                    .collect();
                ALL_TESTS.iter().filter(|t| selected_filters.contains(&t.category_filter)).count()
            }
            Mode::Stress => {
                self.stress_checked.iter().filter(|&c| *c).count()
            }
        }
    }

    fn start_tests(&mut self, rt: &Runtime) {
        // Abort any previously running task
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
                let cats: Vec<String> = CATEGORIES.iter().enumerate()
                    .filter(|(i, _)| self.checked[*i])
                    .flat_map(|(_, c)| c.filters.iter().map(|f| f.to_string()))
                    .collect();

                if cats.is_empty() {
                    self.output_lines.push("[WARN] No categories selected. Press B to go back.".to_string());
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
                    let mut passed = 0u32;
                    let mut failed = 0u32;

                    for test in ALL_TESTS.iter().filter(|t| cats.iter().any(|c| c == t.category_filter)) {
                        if test.category != current_cat {
                            current_cat = test.category;
                            let _ = tx_clone.send(UiMsg::Line(format!("\n[{}]", test.category)));
                        }
                        match run_test(test.name, &mut client).await {
                            Ok(()) => {
                                let _ = tx_clone.send(UiMsg::Line(format!("  [PASS] {}", test.name)));
                                passed += 1;
                            }
                            Err(e) => {
                                let _ = tx_clone.send(UiMsg::Line(format!("  [FAIL] {}", test.name)));
                                let _ = tx_clone.send(UiMsg::Line(format!("         {e}")));
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

// ── UI rendering ────────────────────────────────────────────────────────────

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

    // ── Title with mode tabs ────────────────────────────────────────────

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

    // ── Content ─────────────────────────────────────────────────────────

    match app.mode {
        Mode::Functional => {
            let items: Vec<ListItem> = CATEGORIES
                .iter()
                .enumerate()
                .map(|(i, cat)| {
                    let check = if app.checked[i] { "[x]" } else { "[ ]" };
                    let count = ALL_TESTS.iter().filter(|t| t.category_filter == cat.filters[0]).count();
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {}  {:<12}", check, cat.name),
                            Style::default().fg(if app.checked[i] {
                                Color::Green
                            } else {
                                Color::Gray
                            }),
                        ),
                        Span::styled(
                            format!("  ({})", cat.stages),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("  {} tests", count),
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

    // ── Summary ─────────────────────────────────────────────────────────

    let summary_text = match app.mode {
        Mode::Functional => {
            let n = app.checked.iter().filter(|&c| *c).count();
            format!(" Total: {} tests selected ({} categories)", app.selected_count(), n)
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

    // ── Help ────────────────────────────────────────────────────────────

    let help = match app.mode {
        Mode::Functional => Paragraph::new(Line::from(vec![
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

// ── Main ────────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let rt = Runtime::new().unwrap();

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
                        KeyCode::Up | KeyCode::Char('k') => {
                            let i = app.list_state.selected().unwrap_or(0);
                            if i > 0 {
                                app.list_state.select(Some(i - 1));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max = match app.mode {
                                Mode::Functional => CATEGORIES.len(),
                                Mode::Stress => BENCHMARKS.len(),
                            };
                            let i = app.list_state.selected().unwrap_or(0);
                            if i + 1 < max {
                                app.list_state.select(Some(i + 1));
                            }
                        }
                        KeyCode::Enter => app.start_tests(&rt),
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
