// src/tui/ui.rs

use std::error::Error;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame, Terminal,
};

use crate::stat::Ledger;
use super::app::{App, Screen, InputMode};

/// Entry point for the TUI. Called from main.rs.
pub fn run_tui(ledger: Ledger) -> Result<(), Box<dyn Error>> {
    let mut app = App::new(ledger);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                handle_key_event(&mut app, key);
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Dispatch keyboard events depending on input mode.
fn handle_key_event(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    match app.input_mode {
        InputMode::Normal => handle_key_normal(app, key),
        InputMode::EditingReconcile => handle_key_reconcile_input(app, key),
    }
}

/// Key handling in normal mode.
fn handle_key_normal(app: &mut App, key: KeyEvent) {
    use KeyCode::*;

    match key.code {
        // Quit
        Char('q') => app.should_quit = true,

        // Screen switch
        Tab => app.next_screen(),
        BackTab => app.prev_screen(),

        // Change month on Dashboard
        Left  => app.prev_month(),
        Right => app.next_month(),

        // Shift global time range
        Char('[') => app.shift_range(-1),
        Char(']') => app.shift_range(1),

        // Move selection in list-based screens
        Up => match app.current_screen {
            Screen::Accounts => {
                if app.selected_account_idx > 0 {
                    app.selected_account_idx -= 1;
                }
            }
            Screen::CategoryStats => {
                if app.selected_category_stats_idx > 0 {
                    app.selected_category_stats_idx -= 1;
                }
            }
            Screen::AccountStats => {
                if app.selected_account_stats_idx > 0 {
                    app.selected_account_stats_idx -= 1;
                }
            }
            _ => {}
        },

        Down => match app.current_screen {
            Screen::Accounts => {
                app.selected_account_idx += 1;
            }
            Screen::CategoryStats => {
                app.selected_category_stats_idx += 1;
            }
            Screen::AccountStats => {
                app.selected_account_stats_idx += 1;
            }
            _ => {}
        },

        // Enter reconcile input mode (only Reconcile screen)
        Char('e') => {
            if let Screen::Reconcile = app.current_screen {
                app.input_mode = InputMode::EditingReconcile;
            }
        }

        // Help screen
        Char('?') => {
            app.current_screen = Screen::Help;
        }

        _ => {}
    }
}

/// editing the external balance in Reconcile screen.
fn handle_key_reconcile_input(app: &mut App, key: KeyEvent) {
    use KeyCode::*;

    match key.code {
        Esc => {
            app.input_mode = InputMode::Normal;
        }

        Enter => {
            app.perform_reconcile();
            app.input_mode = InputMode::Normal;
        }

        Backspace => {
            app.reconcile_external_balance.pop();
        }
        Char(c) => {
            if c.is_ascii_digit() || c == '.' || c == '-' {
                app.reconcile_external_balance.push(c);
            }
        }
        _ => {}
    }
}

/// Top-level UI layout: header, main content, footer.
fn ui(f: &mut Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),     // main
            Constraint::Length(1),  // footer
        ])
        .split(f.area());

    // Header
    let (sy, sm) = app.start_month;
    let (ey, em) = app.end_month;
    let screen_name = match app.current_screen {
        Screen::Dashboard     => "Dashboard",
        Screen::Accounts      => "Accounts",
        Screen::CategoryStats => "Category Stats",
        Screen::AccountStats  => "Account Stats",
        Screen::Trends        => "Trends",
        Screen::Reconcile     => "Reconcile",
        Screen::Help          => "Help",
    };
    let header_text = format!(
        "Rust Finance Tracker - {screen_name}   |   Range: {sy:04}-{sm:02} ~ {ey:04}-{em:02}"
    );
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Main content
    match app.current_screen {
        Screen::Dashboard     => draw_dashboard(f, chunks[1], app),
        Screen::Accounts      => draw_accounts(f, chunks[1], app),
        Screen::CategoryStats => draw_category_stats(f, chunks[1], app),
        Screen::AccountStats  => draw_account_stats(f, chunks[1], app),
        Screen::Trends        => draw_trends(f, chunks[1], app),
        Screen::Reconcile     => draw_reconcile(f, chunks[1], app),
        Screen::Help          => draw_help(f, chunks[1], app),
    }

    // Footer
    let footer_text = match app.input_mode {
        InputMode::Normal => {
            "Tab/Shift+Tab: switch screen  |  ←/→: month  |  [ ]: shift range  |  ↑/↓: move  |  e: edit external balance (Reconcile)  |  ?: help  |  q: quit"
        }
        InputMode::EditingReconcile => {
            "Editing external balance: 0-9 . - to type, Enter to submit, Esc to cancel"
        }
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}

// Dashboard screen
fn draw_dashboard(f: &mut Frame<'_>, area: Rect, app: &App) {
    let (year, month) = app.selected_month;
    let user_id = app.user_id;

    let income = app.ledger.month_summary(
        user_id,
        year,
        month,
        None,
        None,
        Some(false),
        None,
    );
    let outcome = app.ledger.month_summary(
        user_id,
        year,
        month,
        None,
        None,
        Some(true),
        None,
    );
    let net = app.ledger.month_summary(
        user_id,
        year,
        month,
        None,
        None,
        None,
        None,
    );

    let text = format!(
        "Focused Month: {year:04}-{month:02}\n\
         Income:  {income:.2}\n\
         Outcome: {outcome:.2}\n\
         Net:     {net:.2}\n"
    );

    let block = Block::default()
        .title(Span::raw("Monthly Summary"))
        .borders(Borders::ALL);

    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}

// Accounts screen, lists all accounts with current balances.
fn draw_accounts(f: &mut Frame<'_>, area: Rect, app: &App) {
    let accounts = app.ledger.all_account_summary();

    let mut selected_idx = app.selected_account_idx;
    if !accounts.is_empty() && selected_idx >= accounts.len() {
        selected_idx = accounts.len() - 1;
    }

    let rows = accounts
        .iter()
        .enumerate()
        .map(|(idx, acc)| {
            let acc_type_str = format!("{:?}", acc.account_type);
            let currency_str = format!("{:?}", acc.currency);

            let cells = vec![
                acc.accountid.to_string(),
                acc.name.clone(),
                acc_type_str,
                format!("{:.2}", acc.balance),
                currency_str,
            ];

            let mut row = Row::new(cells);
            if idx == selected_idx {
                row = row.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            row
        });

    let widths = [
        Constraint::Length(4),
        Constraint::Length(16),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["ID", "Name", "Type", "Balance", "Currency"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Accounts")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}

// CategoryStats screen, shows top categories by outcome
fn draw_category_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
    use crate::stat::datatype::CategoryId;

    let timephase = (app.start_month, app.end_month);
    let trend = app
        .ledger
        .top_category(app.user_id, timephase, None, 10, Some(true)); // rank by outcome

    let norm = trend.normalize();
    // (name, income, outcome, net, percentage_of_spend)
    let mut data: Vec<(String, f64, f64, f64, f64)> = Vec::new();
    for i in 0..trend.axis.len() {
        let cat_id: CategoryId = trend.axis[i];
        let name = app
            .ledger
            .category
            .iter()
            .find(|c| c.id == cat_id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("{:?}", cat_id));
        let income = trend.income[i];
        let outcome = trend.outcome[i];
        let net = trend.summary[i];
        let perc = norm.outcome[i].abs() * 100.0;

        data.push((name, income, outcome, net, perc));
    }

    let mut selected_idx = app.selected_category_stats_idx;
    if !data.is_empty() && selected_idx >= data.len() {
        selected_idx = data.len() - 1;
    }

    let rows = data
        .into_iter()
        .enumerate()
        .map(|(idx, (name, inc, out, net, pct))| {
            let cells = vec![
                format!("{}", idx + 1),
                name,
                format!("{inc:.2}"),
                format!("{out:.2}"),
                format!("{net:.2}"),
                format!("{pct:.1}%"),
            ];
            let mut row = Row::new(cells);
            if idx == selected_idx {
                row = row.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            row
        });

    let widths = [
        Constraint::Length(4),
        Constraint::Length(18),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["#", "Category", "Income", "Outcome", "Net", "% Spend"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Top Categories by Outcome")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}

// AccountStats screen, shows top accounts by outcome.
fn draw_account_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
    use crate::stat::datatype::AccountId;

    let timephase = (app.start_month, app.end_month);
    let trend = app
        .ledger
        .top_account(app.user_id, timephase, None, 10, Some(true)); // rank by outcome

    let norm = trend.normalize();

    let mut data: Vec<(String, f64, f64, f64, f64)> = Vec::new();
    for i in 0..trend.axis.len() {
        let acc_id: AccountId = trend.axis[i];
        let name = app
            .ledger
            .account
            .iter()
            .find(|a| a.id == acc_id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| format!("{:?}", acc_id));
        let inc = trend.income[i];
        let out = trend.outcome[i];
        let net = trend.summary[i];
        let pct = norm.outcome[i].abs() * 100.0;

        data.push((name, inc, out, net, pct));
    }

    let mut selected_idx = app.selected_account_stats_idx;
    if !data.is_empty() && selected_idx >= data.len() {
        selected_idx = data.len() - 1;
    }

    let rows = data
        .into_iter()
        .enumerate()
        .map(|(idx, (name, inc, out, net, pct))| {
            let cells = vec![
                format!("{}", idx + 1),
                name,
                format!("{inc:.2}"),
                format!("{out:.2}"),
                format!("{net:.2}"),
                format!("{pct:.1}%"),
            ];
            let mut row = Row::new(cells);
            if idx == selected_idx {
                row = row.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            row
        });

    let widths = [
        Constraint::Length(4),
        Constraint::Length(18),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["#", "Account", "Income", "Outcome", "Net", "% Spend"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Top Accounts by Outcome")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}

// Trends screen, shows monthly income / outcome / net
fn draw_trends(f: &mut Frame<'_>, area: Rect, app: &App) {
    let timephase = (app.start_month, app.end_month);
    let trend = app
        .ledger
        .data_linetrend(app.user_id, timephase, None, None);

    let mut data: Vec<(String, f64, f64, f64)> = Vec::new();
    for i in 0..trend.axis.len() {
        let (y, m) = trend.axis[i];
        let ym = format!("{y:04}-{m:02}");
        let inc = trend.income[i];
        let out = trend.outcome[i];
        let net = trend.summary[i];
        data.push((ym, inc, out, net));
    }

    let rows = data.into_iter().map(|(ym, inc, out, net)| {
        let cells = vec![
            ym,
            format!("{inc:.2}"),
            format!("{out:.2}"),
            format!("{net:.2}"),
        ];
        Row::new(cells)
    });

    let widths = [
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Month", "Income", "Outcome", "Net"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Monthly Trends")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}

// Reconcile screen, allows entering an external balance and shows:
// internal vs external balance difference
// top suspicious entries explaining the mismatch.
fn draw_reconcile(f: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    // Upper panel: input + reconcile result
    let mut text = String::new();
    let (sy, sm) = app.start_month;
    let (ey, em) = app.end_month;

    text.push_str(&format!(
        "Time range: {sy:04}-{sm:02} ~ {ey:04}-{em:02}\n"
    ));
    text.push_str(&format!(
        "External balance: {}\n",
        app.reconcile_external_balance
    ));

    if let Some(ref view) = app.reconcile_result {
        text.push_str(&format!(
            "Internal: {:+.2}   External: {:+.2}   Diff: {:+.2}\nStatus: {}\n",
            view.internal_balance,
            view.external_balance,
            view.difference,
            if view.good { "OK ✅" } else { "MISMATCH ❌" },
        ));
    } else {
        text.push_str("No reconcile result yet. Press 'e' to edit external balance.\n");
    }

    let block = Block::default()
        .title(Span::raw("Reconcile"))
        .borders(Borders::ALL);
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, chunks[0]);

    // Lower panel: suspicious entries table
    if let Some(ref view) = app.reconcile_result {
        let rows = view.entries.iter().map(|e| {
            let cells = vec![
                e.entry_id.clone(),
                e.date.clone(),
                e.account_name.clone(),
                e.category_name.clone(),
                format!("{:.2}", e.amount),
                e.desc.clone(),
            ];
            Row::new(cells)
        });

        let widths = [
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(10),
            Constraint::Min(10),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["ID", "Date", "Account", "Category", "Amount", "Desc"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Suspicious Entries")
                    .borders(Borders::ALL),
            );

        f.render_widget(table, chunks[1]);
    }
}

//  Help screen
fn draw_help(f: &mut Frame<'_>, area: Rect, _app: &App) {
    let text = "\
Screens:
  Dashboard     – overview of monthly income/expense/net
  Accounts      – list of accounts with balances
  CategoryStats – top spending categories over selected period
  AccountStats  – top spending accounts over selected period
  Trends        – monthly trends of income, outcome, and net
  Reconcile     – compare internal balance with external statement

Key bindings:
  Tab / Shift+Tab : switch screen
  ← / →           : change focused month (Dashboard)
  [ / ]           : shift global time range
  ↑ / ↓           : move selection in lists
  e               : edit external balance (Reconcile)
  ?               : open this help
  q               : quit
";

    let block = Block::default()
        .title(Span::raw("Help"))
        .borders(Borders::ALL);
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}
