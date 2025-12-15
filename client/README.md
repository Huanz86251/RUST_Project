# Rust Finance Tracker - TUI (Ratatui)

Command-line TUI client for the finance tracker backend. Supports full CRUD over HTTP plus multi-entry transactions.

## Prerequisites
- Rust toolchain
- Backend running (default: `http://127.0.0.1:8080`)
- Unless nesserary, you don't need to run it, the databse is already on cloud, database runs local may have compatibility issues for docker and database, to fix that please check backend/README.md. We recommend to skip local compilation of backend.
## Run
```bash
# start backend
cd backend
cargo run

# start TUI client
cd ../client
cargo run --release --bin client
```
## Optional: GPU Acceleration (CUDA / Metal)
The AI Advisor uses local LLM inference (Candle + GGUF).
- By default cargo run, it runs on CPU, which can be slow, if has to run on CPU, please degrade model size to 1.5B or 0.5B.
- You can enable GPU acceleration via Cargo features:

### cuda-NVIDIA GPU:
```bash
cargo run --release --bin client --features cuda
```
### metal-macOS GPU
```bash
cargo run --release --bin client --features metal
```

## Screens & Navigation
- `Tab` / `Shift+Tab`: cycle screens (Dashboard → Accounts → Transactions → CategoryStats → AccountStats → Trends → Reconcile → Advisor → Help)
- `↑` / `↓`: move selection in lists
- `q`: quit
- `?`: help
- `r`: refresh data from server

### Dashboard
- `←` / `→`: change focused month
- `[` / `]`: shift global date range
- `n`: new transaction

### Accounts
- `↑` / `↓`: select account
- `n`: new transaction
- `c`: create account
- `d`: delete first transaction of selected account

### Transactions
- `↑` / `↓`: select transaction
- `n`: new transaction

### Reconcile
- `e`: edit external balance (type numbers), `Enter` submit, `Esc` cancel

## Create Transaction (supports multiple entries)
Enter with `n` (Dashboard/Accounts/Transactions). Fields in order: Date → Payee → Memo → Amount → Account → Category → Entries.

- `Tab` / `Shift+Tab`: switch fields
- Account field: `j/k` switch account
- Category field: `j/k` switch category, `n` create new category
- Entries field:
  - `a`: add current entry (uses Amount/Account/Category)
  - `x`: delete selected entry
  - `j/k`: select entry
- `Enter`: submit (requires at least one entry; if Amount not empty, it is added as an entry on submit)
- `Esc`: cancel

## Create Category
While in Category field press `n`, type name, `Enter` submit (`Esc` cancel). Auto-refresh selects the new category.

## Create Account (Accounts screen, press `c`)
Fields: Name → Type → Currency → Opening Balance
- `Tab` / `Shift+Tab`: switch fields
- Type field: `j/k` cycle (Checking/Credit/Cash/Other)
- `Enter`: submit, `Esc`: cancel

## Delete Transaction
In Accounts screen, select account, press `d` (removes the first transaction of that account). Press `r` to refresh view.

## Advisor (AI Assistant)
AI-powered advisor that analyzes your recent spending and can answer questions / record transactions.

- **Generate summary advice (top panel)**  
  - Go to `Advisor` screen.  
  - `g`: generate a short English analysis based on the last 3 months and top 3 categories.  
  - `m`: change model (Qwen2.5 0.5B / 1.5B / 3B / 7B).  
    - In model select mode: `↑/↓` choose, `m` or `Enter` confirm, `Esc` cancel.
  - `↑ / ↓` (when not selecting model): scroll the Advisor output.

- **Chat with the advisor (bottom panel)**  
  - `i`: enter chat input mode on the Advisor screen.  
  - Type a question in English, `Enter` to send, `Esc` to cancel.  
  - The model uses tools (month summary, top categories/accounts, trends, upload transaction) to answer.  
  - Chat history scroll: `PageUp` / `PageDown` to move through older messages.

## Notes
- Footer shows context-specific shortcuts; errors/success messages appear near footer or panels.
- Backend endpoints: see `backend/README.md`.
