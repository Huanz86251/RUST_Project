use crate::advisor::Modeltype;
use crate::stat::Ledger;
use crate::stat::datatype::{Entry, UserId};
use chrono::{Datelike, Local};

#[derive(Copy, Clone, Debug)]
pub enum Screen {
    Dashboard,
    Accounts,
    Transactions,
    CategoryStats,
    AccountStats,
    Trends,
    Reconcile,
    Advisor,
    Help,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingReconcile,
    CreatingTransaction,
    CreatingCategory,
    CreatingAccount,
    AdvisorChat,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoginMode {
    Email,
    Password,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoginStep {
    Choose,
    Login,
    Register,
}

pub struct LoginApp {
    pub base_url: String,
    pub email: String,
    pub password: String,
    pub mode: LoginMode,
    pub step: LoginStep,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReconcileEntryView {
    pub entry_id: String,
    pub date: String,
    pub account_name: String,
    pub category_name: String,
    pub amount: f64,
    pub desc: String,
}

#[derive(Clone, Debug)]
pub struct ReconcileView {
    pub good: bool,
    pub internal_balance: f64,
    pub external_balance: f64,
    pub difference: f64,
    pub entries: Vec<ReconcileEntryView>,
}

pub struct App {
    pub ledger: Ledger,
    pub user_id: UserId,
    pub current_screen: Screen,
    pub start_month: (i32, u32),
    pub end_month: (i32, u32),
    pub selected_month: (i32, u32),
    pub selected_account_idx: usize,
    pub selected_transaction_idx: usize,
    pub selected_category_stats_idx: usize,
    pub selected_account_stats_idx: usize,
    pub input_mode: InputMode,
    pub reconcile_external_balance: String,
    pub reconcile_result: Option<ReconcileView>,
    pub should_quit: bool,
    pub base_url: String,
    pub token: String,
    pub needs_refresh: bool,
    pub error_message: Option<String>,
    pub new_tx_date: String,
    pub new_tx_payee: String,
    pub new_tx_memo: String,
    pub new_tx_amount: String,
    pub new_tx_account_idx: usize,
    pub new_tx_category_idx: usize,
    pub new_tx_field_idx: usize,
    pub new_category_name: String,
    pub is_creating_new_category: bool,
    pub new_tx_entries: Vec<(usize, usize, String)>, // (account_idx, category_idx, amount)
    pub new_tx_selected_entry_idx: usize,
    pub new_account_name: String,
    pub new_account_type_idx: usize, // field index (0=name, 1=type, 2=currency, 3=balance)
    pub new_account_type_selection: usize, // account type selection (0=Checking, 1=Credit, 2=Cash, 3=Other)
    pub new_account_currency: String,
    pub new_account_balance: String,
    pub advisor_model_type: Modeltype,
    pub advisor_model_choice_idx: usize,
    pub advisor_selecting_model: bool,
    pub advisor_prompt: String,
    pub advisor_advice1: String,
    pub advisor_advice2: String,
    pub advisor_scroll: u16,
    pub advisor_chat_input: String,
    pub advisor_chat_history: Vec<String>,
    pub advisor_chat_scroll: u16,
}

fn add_months((y, m): (i32, u32), delta: i32) -> (i32, u32) {
    let total = y * 12 + (m as i32 - 1) + delta;
    let total = total.max(0);

    let new_y = total / 12;
    let new_m = (total % 12) + 1;
    (new_y, new_m as u32)
}

impl App {
    pub fn new(ledger: Ledger, base_url: String, token: String) -> Self {
        let user_id = ledger
            .user
            .first()
            .expect("demo ledger should have at least one user")
            .id;

        let today = Local::now().date_naive();

        let mut min_ym = (today.year(), today.month());
        let mut max_ym = min_ym;

        if let Some(first_tx) = ledger.transaction.first() {
            min_ym = (first_tx.occur_date.year(), first_tx.occur_date.month());
            max_ym = min_ym;
        }

        for tx in &ledger.transaction {
            let ym = (tx.occur_date.year(), tx.occur_date.month());
            if ym < min_ym {
                min_ym = ym;
            }
            if ym > max_ym {
                max_ym = ym;
            }
        }

        if ledger.transaction.is_empty() {
            min_ym = (today.year(), today.month());
            max_ym = min_ym;
        }

        Self {
            ledger,
            user_id,
            base_url,
            token,
            current_screen: Screen::Dashboard,
            start_month: min_ym,
            end_month: max_ym,
            selected_month: max_ym,
            selected_account_idx: 0,
            selected_transaction_idx: 0,
            selected_category_stats_idx: 0,
            selected_account_stats_idx: 0,
            input_mode: InputMode::Normal,
            reconcile_external_balance: String::new(),
            reconcile_result: None,
            should_quit: false,
            needs_refresh: false,
            error_message: None,
            new_tx_date: String::new(),
            new_tx_payee: String::new(),
            new_tx_memo: String::new(),
            new_tx_amount: String::new(),
            new_tx_account_idx: 0,
            new_tx_category_idx: 0,
            new_tx_field_idx: 0,
            new_category_name: String::new(),
            is_creating_new_category: false,
            new_tx_entries: Vec::new(),
            new_tx_selected_entry_idx: 0,
            new_account_name: String::new(),
            new_account_type_idx: 0,
            new_account_type_selection: 0,
            new_account_currency: String::from("USD"),
            new_account_balance: String::new(),
            advisor_model_type: Modeltype::Qwen25_3B,
            advisor_model_choice_idx: 1,
            advisor_selecting_model: false,
            advisor_prompt: String::new(),
            advisor_advice1: String::new(),
            advisor_advice2: String::new(),
            advisor_scroll: 0,
            advisor_chat_input: String::new(),
            advisor_chat_history: Vec::new(),
            advisor_chat_scroll: 0,
        }
    }

    pub fn next_screen(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Dashboard => Screen::Accounts,
            Screen::Accounts => Screen::Transactions,
            Screen::Transactions => Screen::CategoryStats,
            Screen::CategoryStats => Screen::AccountStats,
            Screen::AccountStats => Screen::Trends,
            Screen::Trends => Screen::Reconcile,
            Screen::Reconcile => Screen::Advisor,
            Screen::Advisor => Screen::Help,
            Screen::Help => Screen::Dashboard,
        };
    }

    pub fn prev_screen(&mut self) {
        self.current_screen = match self.current_screen {
            Screen::Dashboard => Screen::Help,
            Screen::Accounts => Screen::Dashboard,
            Screen::Transactions => Screen::Accounts,
            Screen::CategoryStats => Screen::Transactions,
            Screen::AccountStats => Screen::CategoryStats,
            Screen::Trends => Screen::AccountStats,
            Screen::Reconcile => Screen::Trends,
            Screen::Advisor => Screen::Reconcile,
            Screen::Help => Screen::Advisor,
        };
    }

    pub fn next_month(&mut self) {
        self.selected_month = add_months(self.selected_month, 1);
    }

    pub fn prev_month(&mut self) {
        self.selected_month = add_months(self.selected_month, -1);
    }

    pub fn startmonth_range(&mut self, delta: i32) {
        self.start_month = add_months(self.start_month, delta);
    }
    pub fn endmonth_range(&mut self, delta: i32) {
        self.end_month = add_months(self.end_month, delta);
    }
    pub fn perform_reconcile(&mut self) {
        let trimmed = self.reconcile_external_balance.trim();
        if trimmed.is_empty() {
            self.reconcile_result = None;
            return;
        }

        let Ok(external) = trimmed.parse::<f64>() else {
            // illegal input
            self.reconcile_result = None;
            return;
        };

        let timephase = (self.start_month, self.end_month);
        let res = self
            .ledger
            .reconcile(self.user_id, None, external, timephase, 10);

        let mut entries_view = Vec::new();
        for e in &res.suspicous_entry {
            entries_view.push(self.build_reconcile_entry_view(e));
        }

        self.reconcile_result = Some(ReconcileView {
            good: res.good,
            internal_balance: res.internal_balance,
            external_balance: res.external_balance,
            difference: res.difference,
            entries: entries_view,
        });
    }

    fn build_reconcile_entry_view(&self, entry: &Entry) -> ReconcileEntryView {
        // date
        let date = self
            .ledger
            .transaction
            .iter()
            .find(|t| t.id == entry.tranid)
            .map(|t| format!("{}", t.occur_date))
            .unwrap_or_else(|| "-".to_string());

        // account
        let account_name = self
            .ledger
            .account
            .iter()
            .find(|a| a.id == entry.accountid)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| format!("Account {:?}", entry.accountid));

        // category
        let category_name = match entry.categoryid {
            Some(cat_id) => self
                .ledger
                .category
                .iter()
                .find(|c| c.id == cat_id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("Category {:?}", cat_id)),
            None => "Uncategorized".to_string(),
        };

        // description
        let desc = entry.desc.clone().unwrap_or_default();

        ReconcileEntryView {
            entry_id: format!("{}", entry.id),
            date,
            account_name,
            category_name,
            amount: entry.amount,
            desc,
        }
    }
}
