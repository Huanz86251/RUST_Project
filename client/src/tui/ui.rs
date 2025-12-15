// src/tui/ui.rs
use rust_decimal::prelude::FromPrimitive;

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
use anyhow; // new implemented

/// Entry point for the TUI. Called from main.rs.
pub fn run_tui(ledger: Ledger, base_url: String, token: String) -> anyhow::Result<()> {
    let mut app = App::new(ledger, base_url, token);
    // new implemented 
    let rt = tokio::runtime::Runtime::new()?;

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

        if app.needs_refresh { // new implemented
            app.needs_refresh = false;
            match rt.block_on(
                crate::stat::sync::download_ledger_from_server(&app.base_url, &app.token)
            ) {
                Ok(new_ledger) => {
                    // new implemente
                    if app.is_creating_new_category && !app.new_category_name.trim().is_empty() {
                        if let Some(new_cat) = new_ledger.category.iter()
                            .find(|c| c.name == app.new_category_name.trim())
                        {
                            app.new_tx_category_idx = new_ledger.category.iter()
                                .position(|c| c.id == new_cat.id)
                                .unwrap_or(0);
                        }
                        app.is_creating_new_category = false;
                        app.new_category_name = String::new();
                    }
                    app.ledger = new_ledger;
                    app.success_message = Some("Data refreshed".to_string());
                }
                Err(e) => {
                    app.error_message = Some(format!("Refresh failed: {}", e));
                }
            }
        }

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                handle_key_event(&mut app, key, &rt);
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Dispatch keyboard events depending on input mode.
fn handle_key_event(app: &mut App, key: KeyEvent, rt: &tokio::runtime::Runtime) { // new implemented
    if key.kind != KeyEventKind::Press {
        return;
    }

    match app.input_mode {
        InputMode::Normal => handle_key_normal(app, key, rt),
        InputMode::EditingReconcile => handle_key_reconcile_input(app, key),
        InputMode::CreatingTransaction => handle_key_create_tx(app, key, rt),
        InputMode::CreatingCategory => handle_key_create_category(app, key, rt),
        InputMode::CreatingAccount => handle_key_create_account(app, key, rt), // new implemented
    }
}

/// Key handling in normal mode.
fn handle_key_normal(app: &mut App, key: KeyEvent, rt: &tokio::runtime::Runtime) { // new implemented
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
            Screen::Transactions => {
                if app.selected_transaction_idx > 0 {
                    app.selected_transaction_idx -= 1;
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
            Screen::Transactions => {
                app.selected_transaction_idx += 1;
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

        // new implemented
        Char('r') => {
            app.needs_refresh = true;
        }

        Char('n') => {
            if matches!(app.current_screen, Screen::Dashboard | Screen::Accounts | Screen::Transactions) {
                app.input_mode = InputMode::CreatingTransaction;
                app.new_tx_date = chrono::Local::now().format("%Y-%m-%d").to_string();
                app.new_tx_payee = String::new();
                app.new_tx_memo = String::new();
                app.new_tx_amount = String::new();
                app.new_tx_field_idx = 0;
                app.new_tx_entries = Vec::new(); // new implemented
                app.new_tx_selected_entry_idx = 0; // new implemented
            }
        }

        Char('d') => {
            if let Screen::Accounts = app.current_screen {
                if let Some(account) = app.ledger.account.get(app.selected_account_idx) {
                    if let Some(entry) = app.ledger.entry.iter()
                        .find(|e| e.accountid == account.id)
                    {
                        if let Some(tx) = app.ledger.transaction.iter()
                            .find(|t| t.id == entry.tranid)
                        {
                            match rt.block_on(
                                crate::stat::sync::delete_transaction_on_server(
                                    &app.base_url,
                                    &app.token,
                                    tx.id
                                )
                            ) {
                                Ok(_) => {
                                    app.needs_refresh = true;
                                    app.success_message = Some("Transaction deleted".to_string());
                                }
                                Err(e) => {
                                    app.error_message = Some(format!("Delete failed: {}", e));
                                }
                            }
                        }
                    }
                }
            }
        }

        Char('c') => {
            if matches!(app.current_screen, Screen::Accounts) && app.input_mode == InputMode::Normal {
                app.input_mode = InputMode::CreatingAccount;
                app.new_account_name = String::new();
                app.new_account_type_idx = 0;
                app.new_account_type_selection = 0;
                app.new_account_currency = String::from("USD");
                app.new_account_balance = String::new();
            } else {
                app.error_message = None;
                app.success_message = None;
            }
        }

        _ => {}
    }
}

// new implemented
fn handle_key_create_tx(app: &mut App, key: KeyEvent, rt: &tokio::runtime::Runtime) {
    use KeyCode::*;

    match key.code {
        Esc => {
            app.input_mode = InputMode::Normal;
            app.error_message = None;
        }
        Tab => {
            app.new_tx_field_idx = (app.new_tx_field_idx + 1) % 7;
        }
        BackTab => {
            app.new_tx_field_idx = if app.new_tx_field_idx == 0 { 6 } else { app.new_tx_field_idx - 1 }; // new implemented
        }
        Char('a') => { // new implemented: add entry
            if app.new_tx_amount.trim().is_empty() {
                app.error_message = Some("Amount is required".to_string());
            } else {
                let _amount: f64 = match app.new_tx_amount.trim().parse() {
                    Ok(a) if a > 0.0 => a,
                    _ => {
                        app.error_message = Some("Amount must be a positive number".to_string());
                        return;
                    }
                };
                app.new_tx_entries.push((app.new_tx_account_idx, app.new_tx_category_idx, app.new_tx_amount.clone()));
                app.new_tx_amount.clear();
                app.new_tx_account_idx = 0;
                app.new_tx_category_idx = 0;
                app.error_message = None;
            }
        }
        Char('x') => { // new implemented: delete selected entry
            if !app.new_tx_entries.is_empty() && app.new_tx_selected_entry_idx < app.new_tx_entries.len() {
                app.new_tx_entries.remove(app.new_tx_selected_entry_idx);
                if app.new_tx_selected_entry_idx >= app.new_tx_entries.len() && !app.new_tx_entries.is_empty() {
                    app.new_tx_selected_entry_idx = app.new_tx_entries.len() - 1;
                }
            }
        }
        Enter => {
            // new implemented - 验证必填字段
            if app.new_tx_date.trim().is_empty() {
                app.error_message = Some("Date is required".to_string());
            } else if app.new_tx_entries.is_empty() && app.new_tx_amount.trim().is_empty() {
                app.error_message = Some("At least one entry is required (press 'a' to add entry)".to_string());
            } else if let Err(e) = submit_new_transaction(app, rt) {
                app.error_message = Some(format!("Failed: {}", e));
            } else {
                app.input_mode = InputMode::Normal;
                app.needs_refresh = true;
                app.success_message = Some("Transaction created".to_string());
            }
        }
        Backspace => {
            match app.new_tx_field_idx {
                0 => { app.new_tx_date.pop(); }
                1 => { app.new_tx_payee.pop(); }
                2 => { app.new_tx_memo.pop(); }
                3 => { app.new_tx_amount.pop(); }
                _ => {}
            }
        }
        Char(c) => {
            match app.new_tx_field_idx {
                0 => { app.new_tx_date.push(c); }
                1 => { app.new_tx_payee.push(c); }
                2 => { app.new_tx_memo.push(c); }
                3 => { if c.is_ascii_digit() || c == '.' || c == '-' { app.new_tx_amount.push(c); } }
                4 => {
                    if c == 'j' && app.new_tx_account_idx > 0 {
                        app.new_tx_account_idx -= 1;
                    } else if c == 'k' {
                        app.new_tx_account_idx = (app.new_tx_account_idx + 1).min(app.ledger.account.len().saturating_sub(1));
                    }
                }
                5 => {
                    if c == 'n' {
                        // 进入创建新分类模式
                        app.is_creating_new_category = true;
                        app.input_mode = InputMode::CreatingCategory;
                        app.new_category_name = String::new();
                    } else if c == 'j' && app.new_tx_category_idx > 0 {
                        app.new_tx_category_idx -= 1;
                    } else if c == 'k' {
                        let max_idx = app.ledger.category.len();
                        app.new_tx_category_idx = (app.new_tx_category_idx + 1).min(max_idx);
                    }
                }
                6 => { // new implemented: navigate entries list
                    if c == 'j' && app.new_tx_selected_entry_idx > 0 {
                        app.new_tx_selected_entry_idx -= 1;
                    } else if c == 'k' {
                        app.new_tx_selected_entry_idx = (app.new_tx_selected_entry_idx + 1).min(app.new_tx_entries.len().saturating_sub(1));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn submit_new_transaction(app: &mut App, rt: &tokio::runtime::Runtime) -> anyhow::Result<()> {
    use crate::stat::sync::*;
    use rust_decimal::Decimal;
    use chrono::NaiveDate;

    if app.new_tx_date.trim().is_empty() {
        return Err(anyhow::anyhow!("Date is required").into());
    }
    let date = NaiveDate::parse_from_str(&app.new_tx_date.trim(), "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date format (use YYYY-MM-DD)"))?;
    
    let mut entries = Vec::new();
    
    // new implemented: add entries from list
    for (account_idx, category_idx, amount_str) in &app.new_tx_entries {
        let amount: f64 = amount_str.trim().parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount in entry: {}", amount_str))?;
        if amount <= 0.0 {
            return Err(anyhow::anyhow!("Amount must be greater than 0").into());
        }
        let account = app.ledger.account.get(*account_idx)
            .ok_or_else(|| anyhow::anyhow!("Invalid account in entry"))?;
        let category_id = if *category_idx >= app.ledger.category.len() {
            None
        } else {
            app.ledger.category.get(*category_idx).map(|c| c.id)
        };
        entries.push(Entryreq {
            account_id: account.id,
            category_id,
            amount: Decimal::from_f64(-amount.abs()).unwrap(),
            note: None,
        });
    }
    
    // new implemented: add current entry if amount is filled
    if !app.new_tx_amount.trim().is_empty() {
        let amount: f64 = app.new_tx_amount.trim().parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount (must be a number)"))?;
        if amount <= 0.0 {
            return Err(anyhow::anyhow!("Amount must be greater than 0").into());
        }
        let account = app.ledger.account.get(app.new_tx_account_idx)
            .ok_or_else(|| anyhow::anyhow!("Invalid account"))?;
        let category_id = if app.is_creating_new_category {
            None
        } else {
            app.ledger.category.get(app.new_tx_category_idx)
                .map(|c| c.id)
        };
        entries.push(Entryreq {
            account_id: account.id,
            category_id,
            amount: Decimal::from_f64(-amount.abs()).unwrap(),
            note: if app.new_tx_memo.is_empty() { None } else { Some(app.new_tx_memo.clone()) },
        });
    }
    
    if entries.is_empty() {
        return Err(anyhow::anyhow!("At least one entry is required").into());
    }

    rt.block_on(
        create_cloudtransaction(
            &app.base_url,
            &app.token,
            date,
            if app.new_tx_payee.is_empty() { None } else { Some(&app.new_tx_payee) },
            if app.new_tx_memo.is_empty() { None } else { Some(&app.new_tx_memo) },
            entries,
        )
    )?;
    Ok(())
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
        Screen::Transactions => "Transactions",
        Screen::CategoryStats => "Category Stats",
        Screen::AccountStats  => "Account Stats",
        Screen::Trends        => "Trends",
        Screen::Reconcile     => "Reconcile",
        Screen::Help          => "Help",
        Screen::Advisor       => "Advisor",
    };
    let header_text = format!(
        "Rust Finance Tracker - {screen_name}   |   Range: {sy:04}-{sm:02} ~ {ey:04}-{em:02}"
    );
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    // Main content
    match app.current_screen {
        Screen::Dashboard => {
            if app.input_mode == InputMode::CreatingCategory {
                draw_create_category(f, chunks[1], app);
            } else if app.input_mode == InputMode::CreatingTransaction {
                draw_create_transaction(f, chunks[1], app);
            } else {
                draw_dashboard(f, chunks[1], app);
            }
        }
        Screen::Accounts => {
            if app.input_mode == InputMode::CreatingAccount {
                draw_create_account(f, chunks[1], app);
            } else if app.input_mode == InputMode::CreatingCategory {
                draw_create_category(f, chunks[1], app);
            } else if app.input_mode == InputMode::CreatingTransaction {
                draw_create_transaction(f, chunks[1], app);
            } else {
                draw_accounts(f, chunks[1], app);
            }
        },
        Screen::Transactions => draw_transactions(f, chunks[1], app),
        Screen::CategoryStats => draw_category_stats(f, chunks[1], app),
        Screen::AccountStats  => draw_account_stats(f, chunks[1], app),
        Screen::Trends        => draw_trends(f, chunks[1], app),
        Screen::Reconcile     => draw_reconcile(f, chunks[1], app),
        Screen::Advisor       => draw_advisor(f, chunks[1], app),
        Screen::Help          => draw_help(f, chunks[1], app),
    }

    // Footer
    let footer_text = if let Some(ref msg) = app.error_message {
        format!("ERROR: {} | Press 'c' to clear", msg)
    } else if let Some(ref msg) = app.success_message {
        format!("SUCCESS: {} | Press 'c' to clear", msg)
    } else {
        match app.input_mode {
            InputMode::Normal => {
                "Tab/Shift+Tab: switch screen  |  ←/→: month  |  [ ]: shift range  |  ↑/↓: move  |  r: refresh  |  n: new transaction  |  d: delete  |  e: edit external balance (Reconcile)  |  ?: help  |  q: quit".to_string()
            }
            InputMode::EditingReconcile => {
                "Editing external balance: 0-9 . - to type, Enter to submit, Esc to cancel".to_string()
            }
            InputMode::CreatingTransaction => {
                "Creating transaction: Tab to switch fields, Enter to submit, Esc to cancel".to_string()
            }
            InputMode::CreatingCategory => {
                "Creating category: Type name, Enter to submit, Esc to cancel".to_string()
            }
            InputMode::CreatingAccount => {
                "Creating account: Tab to switch fields, j/k to change type, Enter to submit, Esc to cancel".to_string()
            }
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

    let timephase = (app.start_month, app.end_month);
    let trend = app
        .ledger
        .top_category(app.user_id, timephase, None, 10, Some(true)); // rank by outcome

    let norm = trend.normalize();
    // (name, income, outcome, net, percentage_of_spend)
    let mut data: Vec<(String, f64, f64, f64, f64)> = Vec::new();
    for i in 0..trend.axis.len() {
        let name = trend.axis[i].clone(); 
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

// new implemented
fn draw_transactions(f: &mut Frame<'_>, area: Rect, app: &App) {
    let transactions = &app.ledger.transaction;
    
    let mut selected_idx = app.selected_transaction_idx;
    if !transactions.is_empty() && selected_idx >= transactions.len() {
        selected_idx = transactions.len() - 1;
    }

    let rows = transactions
        .iter()
        .enumerate()
        .map(|(idx, tx)| {
            let total_amount: f64 = app.ledger.entry
                .iter()
                .filter(|e| e.tranid == tx.id)
                .map(|e| e.amount)
                .sum();
            
            let entry_count = app.ledger.entry
                .iter()
                .filter(|e| e.tranid == tx.id)
                .count();
            
            let payee_str = tx.receiver.as_deref().unwrap_or("-");
            let memo_str = tx.desc.as_deref().unwrap_or("-");
            
            let cells = vec![
                format!("{}", tx.occur_date),
                payee_str.to_string(),
                memo_str.to_string(),
                format!("{:.2}", total_amount),
                format!("{}", entry_count),
            ];

            let mut row = Row::new(cells);
            if idx == selected_idx {
                row = row.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            row
        });

    let widths = [
        Constraint::Length(12),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(12),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Date", "Payee", "Memo", "Amount", "Entries"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Transactions")
                .borders(Borders::ALL),
        );

    f.render_widget(table, area);
}

// AccountStats screen, shows top accounts by outcome.
fn draw_account_stats(f: &mut Frame<'_>, area: Rect, app: &App) {

    let timephase = (app.start_month, app.end_month);
    let trend = app
        .ledger
        .top_account(app.user_id, timephase, None, 10, Some(true)); // rank by outcome

    let norm = trend.normalize();

    let mut data: Vec<(String, f64, f64, f64, f64)> = Vec::new();
    for i in 0..trend.axis.len() {
        let name = trend.axis[i].clone(); 
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

fn draw_advisor(f: &mut Frame<'_>, area: Rect, _app: &App) {
    let text = "AI Advisor - Press 'g' to generate advice";
    let block = Block::default()
        .title("AI Financial Advisor")
        .borders(Borders::ALL);
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}

// new implemented
fn draw_create_transaction(f: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Min(5),
            Constraint::Min(0),
        ])
        .split(area);

    let fields = vec![
        ("Date (YYYY-MM-DD)", &app.new_tx_date, 0),
        ("Payee", &app.new_tx_payee, 1),
        ("Memo", &app.new_tx_memo, 2),
        ("Amount", &app.new_tx_amount, 3),
    ];

    let mut text = String::new();
    for (label, value, idx) in &fields {
        let marker = if *idx == app.new_tx_field_idx { "> " } else { "  " };
        text.push_str(&format!("{}{}: {}\n", marker, label, value));
    }

    let account_marker = if app.new_tx_field_idx == 4 { "> " } else { "  " };
    let account_name = app.ledger.account.get(app.new_tx_account_idx)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "None".to_string());
    text.push_str(&format!("{}Account: {} (j/k to change)\n", account_marker, account_name));

    let category_marker = if app.new_tx_field_idx == 5 { "> " } else { "  " };
    let category_name = if app.new_tx_category_idx >= app.ledger.category.len() {
        "[Create New Category]".to_string()
    } else {
        app.ledger.category.get(app.new_tx_category_idx)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "None (optional)".to_string())
    };
    text.push_str(&format!("{}Category: {} (j/k to change, n to create new)\n", category_marker, category_name));
    
    // new implemented: show entries list
    let entries_marker = if app.new_tx_field_idx == 6 { "> " } else { "  " };
    text.push_str(&format!("{}Entries: {} (a: add, x: delete, j/k: select)", entries_marker, app.new_tx_entries.len()));

    let block = Block::default()
        .title("Create Transaction (Enter to submit, Esc to cancel, a: add entry, x: delete entry)")
        .borders(Borders::ALL);
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, chunks[0]);

    // new implemented: show entries list
    if !app.new_tx_entries.is_empty() {
        let rows = app.new_tx_entries.iter().enumerate().map(|(idx, (acc_idx, cat_idx, amount))| {
            let acc_name = app.ledger.account.get(*acc_idx)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let cat_name = if *cat_idx >= app.ledger.category.len() {
                "None".to_string()
            } else {
                app.ledger.category.get(*cat_idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "None".to_string())
            };
            let mut row = Row::new(vec![
                format!("{}", idx + 1),
                acc_name,
                cat_name,
                amount.clone(),
            ]);
            if idx == app.new_tx_selected_entry_idx {
                row = row.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            row
        });
        let widths = [
            Constraint::Length(4),
            Constraint::Length(16),
            Constraint::Length(16),
            Constraint::Length(12),
        ];
        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["#", "Account", "Category", "Amount"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Entries")
                    .borders(Borders::ALL),
            );
        f.render_widget(table, chunks[1]);
    }

    if let Some(ref msg) = app.error_message {
        let err_block = Block::default()
            .title("Error")
            .borders(Borders::ALL);
        let err_p = Paragraph::new(msg.as_str()).block(err_block);
        f.render_widget(err_p, chunks[2]);
    } else if let Some(ref msg) = app.success_message {
        let succ_block = Block::default()
            .title("Success")
            .borders(Borders::ALL);
        let succ_p = Paragraph::new(msg.as_str()).block(succ_block);
        f.render_widget(succ_p, chunks[2]);
    }
}

// new implemented
fn handle_key_create_account(app: &mut App, key: KeyEvent, rt: &tokio::runtime::Runtime) {
    use KeyCode::*;

    match key.code {
        Esc => {
            app.input_mode = InputMode::Normal;
            app.error_message = None;
        }
        Tab => {
            app.new_account_type_idx = (app.new_account_type_idx + 1) % 4;
        }
        BackTab => {
            app.new_account_type_idx = if app.new_account_type_idx == 0 { 3 } else { app.new_account_type_idx - 1 };
        }
        Enter => {
            app.error_message = None; // new implemented 
            if app.new_account_name.trim().is_empty() {
                app.error_message = Some("Account name is required".to_string());
            } else {
                match submit_new_account(app, rt) {
                    Ok(_) => {
                        app.input_mode = InputMode::Normal;
                        app.needs_refresh = true;
                        app.success_message = Some("Account created".to_string());
                        // new implemented
                        app.new_account_name = String::new();
                        app.new_account_type_idx = 0;
                        app.new_account_type_selection = 0;
                        app.new_account_currency = String::from("USD");
                        app.new_account_balance = String::new();
                    }
                    Err(e) => {
                        app.error_message = Some(format!("Failed: {}", e));
                    }
                }
            }
        }
        Backspace => {
            match app.new_account_type_idx {
                0 => { app.new_account_name.pop(); }
                2 => { app.new_account_currency.pop(); }
                3 => { app.new_account_balance.pop(); }
                _ => {}
            }
        }
        Char(c) => {
            match app.new_account_type_idx {
                0 => { 
                    app.new_account_name.push(c); 
                }
                1 => {
                    if c == 'j' && app.new_account_type_selection > 0 {
                        app.new_account_type_selection -= 1;
                    } else if c == 'k' {
                        app.new_account_type_selection = (app.new_account_type_selection + 1) % 4;
                    }
                }
                2 => {                    app.new_account_currency.push(c); 
                }
                3 => { 
                    if c.is_ascii_digit() || c == '.' || c == '-' { 
                        app.new_account_balance.push(c); 
                    } 
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn submit_new_account(app: &mut App, rt: &tokio::runtime::Runtime) -> anyhow::Result<()> {
    use crate::stat::datatype::AccountType;
    use crate::stat::sync::*;

    let account_type = match app.new_account_type_selection {
        0 => AccountType::Checking,
        1 => AccountType::Credit,
        2 => AccountType::Cash,
        3 => AccountType::Other("other".to_string()),
        _ => AccountType::Checking,
    };

    let currency = if app.new_account_currency.trim().is_empty() {
        None
    } else {
        Some(app.new_account_currency.trim())
    };

    let balance = if app.new_account_balance.trim().is_empty() {
        None
    } else {
        Some(app.new_account_balance.trim().parse::<f64>()
            .map_err(|_| anyhow::anyhow!("Invalid balance (must be a number)"))?)
    };

    rt.block_on(
        create_cloudaccount(
            &app.base_url,
            &app.token,
            &app.new_account_name.trim(),
            &account_type,
            currency,
            balance,
        )
    )?;
    Ok(())
}

fn draw_create_account(f: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    let account_types = vec!["Checking", "Credit", "Cash", "Other"];
    let account_type_name = account_types.get(app.new_account_type_selection)
        .unwrap_or(&"Checking").to_string();

    let fields = vec![
        ("Name", &app.new_account_name, 0),
        ("Type", &account_type_name, 1),
        ("Currency", &app.new_account_currency, 2),
        ("Opening Balance", &app.new_account_balance, 3),
    ];

    let mut text = String::new();
    for (label, value, idx) in &fields {
        let marker = if *idx == app.new_account_type_idx { "> " } else { "  " };
        if *idx == 1 {
            text.push_str(&format!("{}{}: {} (j/k to change)\n", marker, label, value));
        } else {
            text.push_str(&format!("{}{}: {}\n", marker, label, value));
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    let block = Block::default()
        .title("Create Account (Enter to submit, Esc to cancel)")
        .borders(Borders::ALL);
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, chunks[0]);

    // new implemented - show error/success messages
    if let Some(ref msg) = app.error_message {
        let err_block = Block::default()
            .title("Error")
            .borders(Borders::ALL);
        let err_p = Paragraph::new(msg.as_str()).block(err_block);
        f.render_widget(err_p, chunks[1]);
    } else if let Some(ref msg) = app.success_message {
        let succ_block = Block::default()
            .title("Success")
            .borders(Borders::ALL);
        let succ_p = Paragraph::new(msg.as_str()).block(succ_block);
        f.render_widget(succ_p, chunks[1]);
    }
}

// new implemented
fn handle_key_create_category(app: &mut App, key: KeyEvent, rt: &tokio::runtime::Runtime) {
    use KeyCode::*;

    match key.code {
        Esc => {
            app.input_mode = InputMode::CreatingTransaction;
            app.is_creating_new_category = false;
            app.new_category_name = String::new();
            app.error_message = None;
        }
        Enter => {
            if app.new_category_name.trim().is_empty() {
                app.error_message = Some("Category name is required".to_string());
            } else if let Err(e) = submit_new_category(app, rt) {
                app.error_message = Some(format!("Failed: {}", e));
            } else {
                app.is_creating_new_category = false;
                app.input_mode = InputMode::CreatingTransaction;
                app.needs_refresh = true;
                app.success_message = Some("Category created, refreshing...".to_string());
            }
        }
        Backspace => {
            app.new_category_name.pop();
        }
        Char(c) => {
            app.new_category_name.push(c);
        }
        _ => {}
    }
}

fn submit_new_category(app: &mut App, rt: &tokio::runtime::Runtime) -> anyhow::Result<()> {
    use crate::stat::sync::*;
    
    rt.block_on(
        create_cloudcate(
            &app.base_url,
            &app.token,
            &app.new_category_name.trim(),
            None,
        )
    )?;
    Ok(())
}

// new implemented
fn draw_create_category(f: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    let mut text = String::new();
    text.push_str("Category Name: ");
    text.push_str(&app.new_category_name);
    text.push_str("\n\nPress Enter to create, Esc to cancel");

    let block = Block::default()
        .title("Create New Category")
        .borders(Borders::ALL);
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, chunks[0]);

    if let Some(ref msg) = app.error_message {
        let err_block = Block::default()
            .title("Error")
            .borders(Borders::ALL);
        let err_p = Paragraph::new(msg.as_str()).block(err_block);
        f.render_widget(err_p, chunks[1]);
    } else if let Some(ref msg) = app.success_message {
        let succ_block = Block::default()
            .title("Success")
            .borders(Borders::ALL);
        let succ_p = Paragraph::new(msg.as_str()).block(succ_block);
        f.render_widget(succ_p, chunks[1]);
    }
}
