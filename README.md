# Project Proposal: Rust Web Crawler with Data Analysis
## Group Members

| Role | Name | Student ID | GitHub ID |preferred email addresses
|------|------|------------|----------|-----------------|
| **Member A** | Zihao Gong | 1005036916 | [Zihao1121](https://github.com/Zihao1121) | zihao.gong@mail.utoronto.ca|
| **Member B** | Shiming Zhang | 1011821129 | [Ming031121](https://github.com/Ming031121) | shim.zhang@mail.utoronto.ca|
| **Member C** | Zixuan Huang | 1006288376 | [Huanz86251](https://github.com/Huanz86251) | chrim.huang@mail.utoronto.ca|

## Motivation
In daily life, everyone needs to manage multiple accounts, such as checking accounts and credit cards. Each transaction and income often has a different purpose and corresponding date. And it is difficult to use mobile banking apps or spreadsheets to achieve granular statistical analysis. At the same time, many existing personal finance management apps only record transaction history but lack reconciliation processes. If discrepancies arise between internally recorded balances and bank/credit card statement balances, it is hard for users to pinpoint the source of the problem. Furthermore, in today's LLM-driven world, apps without integrated large-scale models are a little bit outdated. And some projects that use LLM tend to use cloud-based models, with few willing to invest in implementing local deployments and inference logic for LLM.

Therefore, we aim to build a personal finance tool that utilizes local LLM and emphasizes statistics and analysis: a keyboard-friendly TUI front-end, and a back-end providing unified data storage and synchronization via HTTPS.

---

## Objective and Key Features
### Objective 

The project we designed and built is a personal finance tracker centred on a TUI client, LLM and a secure HTTPS backend API. The APP should help users record their income/expenses, organize them into categories and accounts (e.g., checking/credit/cash), and manage complex transactions that span multiple accounts, categories, and entries. In addition to day-to-day bookkeeping, the system supports account reconciliation and provides AI-assisted financial insights and advice.

Unlike many existing Rust CLI finance tools that are either local-file-based (CSV/SQLite) or do not provide an authenticated remote service, our project delivers a full end-to-end workflow: authentication, a structured database schema, a RESTful API, a local LLM to analyze, and an ergonomic TUI for daily use. We support both a local database mode and an HTTPS back-end database mode to satisfy different security and deployment requirements. The remote mode enforces user-scoped access via authentication to ensure data isolation and safe multi-user usage. To reduce friction when handling large finance reports or unstructured information—tasks that are painful to type manually in a TUI Meanwhile, within the RUST ecosystem, we are one of the few personal finance management projects that use locally deployed quantized large-scale models. This greatly reduces the cost of using AI as users do not need to pay for AI APIs, and the agent chain invocation becomes more flexible. AI will provide two suggestions to users based on statistical data from the most recent period. We have also implemented our own LLM agent chain, which can help users directly upload partial records or directly help users analyze data without having to manually click on different functions to view the data. 
Overall, this project fills a gap in the Rust ecosystem by offering a modern, strongly-typed, self-hostable finance tracker that goes beyond simple logging, supporting reconciliation, reporting, and AI-assisted workflows.

### Key Features
**Secure authentication & user isolation**: Register/login with token-based authentication; all data access is scoped per user.
*Value to objective*: ensures user data privacy and enables safe usage on a remote HTTPS service.

**Local + remote database support**: Select local or cloud back-end by configuring .env (base URL / database settings), allowing the same client to run against either environment.
*Value to objective*: supports both offline/local development and real multi-device usage without changing code.

**Accounts management**: Create/list/update accounts (e.g., checking/credit/cash) with opening balances and computed current balances.
*Value to objective*: provides a reliable foundation for tracking money across real-world accounts.

**Categories management**: Create/list/updatedelete categories to organize expenses and income.
*Value to objective*: Categories make transaction history searchable and enable meaningful summaries (e.g., “How much did I spend on groceries this month?”), which is essential for budgeting and reporting.
**Transaction logging**: Users can record transactions and view history through the TUI/API, including descriptions/notes for auditability.
*Value to objective*: enables consistent daily bookkeeping and traceable records.


Complex Split Transactions (Multiple Entries Across Accounts/Categories)
A single transaction can contain multiple entries (splits), allowing one real-world event to be allocated across multiple categories and/or accounts.
*Value to objective*: matches real finance scenarios (e.g., one purchase split across categories) and keeps balances accurate.

**Reconciliation**: Compare computed balances with user-entered statement balances, and if there are discrepancies, we will return the top_k most suspicious transactions.

**LLM-assisted input & analysis**: The LLM can interpret the user's recent statistical data and give two advice. We will record the user's preferred choice and save it to the local folder for culture DPO training. 

**LLM-agent chain**: The LLM can decide if it needs to call agents (5 agents total: Monthly spend/income summary; Recent top spending categories; Recent top spending accounts; Recent month-by-month trends; Action tool: Upload new transaction
) based on the user’s question and analyze the output. 




## User’s (or Developer’s) Guide:
  
  Command-line TUI client for the finance tracker backend. Supports full CRUD over HTTP plus multi-entry transactions.
  
  #### Screens & Navigation
  - `Tab` / `Shift+Tab`: cycle screens (Dashboard → Accounts → Transactions → CategoryStats → AccountStats → Trends → Reconcile → Advisor → Help)
  - `↑` / `↓`: move selection in lists
  - `q`: quit
  - `?`: help
  - `r`: refresh data from server
  
 #### Dashboard
  - `←` / `→`: change focused month
  - `[` / `]`: shift global date range
  - `n`: new transaction
  
  #### Accounts
  - `↑` / `↓`: select account
  - `n`: new transaction
  - `c`: create account
  - `d`: delete first transaction of selected account
  
  #### Transactions
  - `↑` / `↓`: select transaction
  - `n`: new transaction
  
  #### Reconcile
  - `e`: edit external balance (type numbers), `Enter` submit, `Esc` cancel
  
  #### Create Transaction (supports multiple entries)
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
  
  #### Create Category
  While in Category field press `n`, type name, `Enter` submit (`Esc` cancel). Auto-refresh selects the new category.
  
  #### Create Account (Accounts screen, press `c`)
  Fields: Name → Type → Currency → Opening Balance
  - `Tab` / `Shift+Tab`: switch fields
  - Type field: `j/k` cycle (Checking/Credit/Cash/Other)
  - `Enter`: submit, `Esc`: cancel
  
  #### Delete Transaction
  In Accounts screen, select account, press `d` (removes the first transaction of that account). Press `r` to refresh view.
  
  #### Advisor (AI Assistant)
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
### User guide for backend server without client (Only Curl)

#### For developer who want to try server with local database:
cargo run --release
BASE=http://localhost:8080

#### For users  or developer that want to try Database stored in a HTTPS back-end server:
BASE=https://finance-backend.bravestone-51d4c984.canadacentral.azurecontainerapps.io
### And you don't need to cargo run.

#### 1. Register
curl -i -X POST "$BASE/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test1@example.com",
    "password": "TestPass123!"
  }'

#### 2. Login to get JWT – POST /auth/login
#### 2.1 Quickly inspect the response
curl -i -X POST "$BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test1@example.com",
    "password": "TestPass123!"
  }'
#### 2.2 Use jq to extract the token
TOKEN=$(curl -sS -X POST "$BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test1@example.com",
    "password": "TestPass123!"
  }' | jq -r '.token')

echo "$TOKEN"

#### 3. Test root route – GET /
curl -i "$BASE/" \
  -H "Authorization: Bearer $TOKEN"


##### Expected something like:

Hello, user_id=xxxx-...

#### 4. Accounts – /accounts
#### 4.1 Create an account – POST /accounts
curl -i -X POST "$BASE/accounts" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "CIBC Checking",
    "account_type": "checking",
    "currency": "CAD",
    "opening_balance": 1000.00
  }'

#### 4.2 List accounts (simple) – GET /accounts
curl -i "$BASE/accounts" \
  -H "Authorization: Bearer $TOKEN"

#### 4.3 With balance & filters/sorting
curl -i "$BASE/accounts?include_balance=true&limit=50&offset=0&sort=created_at&order=desc" \
  -H "Authorization: Bearer $TOKEN"


##### You can also filter by checking accounts and CAD, for example:

curl -i "$BASE/accounts?type=checking&currency=CAD&include_balance=true" \
  -H "Authorization: Bearer $TOKEN"

#### 5. Categories – /categories
#### 5.1 Create a top-level category – POST /categories
curl -i -X POST "$BASE/categories" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Food",
    "parent_id": null
  }'

#### 5.2 Create a subcategory (assume Food has id = 1)
curl -i -X POST "$BASE/categories" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Restaurant",
    "parent_id": 1
  }'

#### 5.3 List all categories – GET /categories
curl -i "$BASE/categories" \
  -H "Authorization: Bearer $TOKEN"

#### 6. Transactions & entries – /transactions

##### Your request body structure (CreateTransactionsReq in backend) is:

{
  "payee": "optional",
  "memo": "optional",
  "occurred_at": "YYYY-MM-DD",
  "entries": [
    {
      "account_id": 1,
      "category_id": 2,    // can be null
      "amount": -20.50,    // Decimal -> JSON number
      "note": "optional"
    }
  ]
}

#### 6.1 Create a transaction (expense) – POST /transactions

##### Example: on account 1, category 2, spend 20.5:

curl -i -X POST "$BASE/transactions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "payee": "Starbucks",
    "memo": "Latte",
    "occurred_at": "2025-12-10",
    "entries": [
      {
        "account_id": 1,
        "category_id": 2,
        "amount": -5.50,
        "note": "coffee"
      }
    ]
  }'


##### You can also create an “income” transaction:

curl -i -X POST "$BASE/transactions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "payee": "Company",
    "memo": "Salary",
    "occurred_at": "2025-12-01",
    "entries": [
      {
        "account_id": 1,
        "category_id": null,
        "amount": 3000.00,
        "note": "monthly salary"
      }
    ]
  }'

#### 6.2 List transactions – GET /transactions

##### Simple list (default limit/offset):

curl -i "$BASE/transactions" \
  -H "Authorization: Bearer $TOKEN"


##### With pagination parameters:

curl -i "$BASE/transactions?limit=50&offset=0" \
  -H "Authorization: Bearer $TOKEN"


##### Each returned TransactionsDto includes entries: Vec<EntriesDto>.

#### 7. Ledger summary snapshot – /ledger or /ledger/snapshot

#### If you mount the handler on /ledger, it’s roughly like this:

curl -i "$BASE/ledger" \
  -H "Authorization: Bearer $TOKEN"


#### If it’s /ledger/snapshot, then:

curl -i "$BASE/ledger/snapshot" \
  -H "Authorization: Bearer $TOKEN"
 

## Reproducibility Guide

  #### Prerequisites
  - Rust toolchain
  - sudo apt-get update
  - sudo apt-get install -y pkg-config libssl-dev
  #### Optional: GPU Acceleration (CUDA / Metal)
  The AI Advisor uses local LLM inference (Candle + GGUF).
  - By default cargo run, it runs on CPU, which can be slow, if has to run on CPU, please degrade model size to 1.5B or 0.5B.
  - You can enable GPU acceleration via Cargo features:
  
  #### cuda-NVIDIA GPU:
  ```bash
  cargo run --release --bin client --features cuda
  ```
  #### metal-macOS GPU
  ```bash
  cargo run --release --bin client --features metal
  ```
  #### Notes
  - Footer shows context-specific shortcuts; errors/success messages appear near footer or panels.
  - Backend endpoints: see `backend/README.md`.


## Backend Reproducibility Guide for developer
### First clone the git repo, if already cloned, enter backend directionary
git clone https://github.com/Huanz86251/RUST_Project.git
cd RUST_Project/backend
### Then check Rustc version, require at least 1.88
### If not, try to update the rustc.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

rustup update stable
rustup default stable

rustc --version

### Download Docker
#### If already downloaded, pls verify

docker --version
docker compose version
sudo docker run --rm hello-world
#### macOS Sonoma
brew install --cask docker

#### Ubuntu system
sudo apt-get remove -y docker.io docker-doc docker-compose podman-docker containerd runc || true

sudo apt-get update
sudo apt-get install -y ca-certificates curl gnupg

sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg

echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
  https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo $VERSION_CODENAME) stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

  sudo apt-get update
  sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

docker --version
docker compose version

#### Debian system
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc

echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] \
https://download.docker.com/linux/debian \
$(. /etc/os-release && echo $VERSION_CODENAME) stable" | \
sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

sudo apt-get update
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

#### Check if Docker daemon is running
sudo systemctl status docker --no-pager
#### If not, run
sudo systemctl enable --now docker
#### Add user into docker group
sudo usermod -aG docker $USER
newgrp docker
#### Now run the docker compose to build the DB.
docker compose up -d

#### it should show STATUS healthy
sudo docker compose ps

### If there's no .env file or you want to create your own .env, run these cmd
JWT_SECRET="$(openssl rand -hex 32)" && cat > .env <<EOF
DATABASE_URL=postgres://finance:finance_pw@localhost:5432/finance
JWT_SECRET=$JWT_SECRET
EOF

### Install sqlx
cargo install sqlx-cli --no-default-features --features postgres --locked

### Build the Sql using SQLx migrations
DATABASE_URL=postgres://finance:finance_pw@localhost:5432/finance sqlx migrate run

### Try to run cargo build
DATABASE_URL=postgres://finance:finance_pw@localhost:5432/finance cargo sqlx prepare
cargo build
    
## Contributions by each team member
**Zihao Gong (Back-end & Database)**: Implemented the HTTPS back-end service, including the overall API structure, database schema design and migrations, and core endpoints for authentication and finance operations. This member focused on ensuring data correctness and security, such as user-scoped access control, validation, and consistent handling of complex split transactions.

**Zixuan Huang (Back-end & Database)**: Implemented the offline LLM integration and the client-side analytics core. This included running quantized Qwen2.5 models locally with Candle (tokenizer/weight loading, device selection, and generation settings), building an LLM tool-calling agent chain, and implementing the Ledger statistics layer (monthly summaries, recent trends, and top category/account ranking). In addition, developed Cloud↔Local data mappings to connect the TUI/AI workflow with the back-end service.

**Shiming Zhang (TUI Client & System Integration)**: Designed and implemented the Ratatui-based TUI client, including the screen layout (Dashboard, Accounts, Transactions, Trends, Reconcile, Advisor, Help) and keyboard navigation. Implemented HTTP client integration in the TUI (fetching and mutating data via the back-end API), including creating/deleting transactions, creating accounts/categories, and handling error/success messages. Built the AI Advisor TUI features (model selection, advice generation, chat interface with scrolling)

## Lessons learned and concluding remarks
Through this project, we learned how to better divide tasks and collaborate, and how teammates can agree on interfaces. For example, our data analysis and TUI teams uniformly used the ledger structure as the central hub, which greatly simplified subsequent integration. Backend-client integration only required writing a mapping structure. We also learned how to call models, maintain the model's forward process, assemble templates, call agents, and build TUI within the Rust environment. The most direct takeaway was that debugging after a Rust project is completed is indeed easier, but the development process has a higher upfront development cost. This project helped us learn how to develop an end-to-end Rust project.
### Innovation
Most student-scale finance trackers that add AI features rely on third-party APIs, while they rarely demonstrate a complete offline LLM workflow. We load quantized large models locally (Candle + GGUF), concatenate prompt word templates, control generation configuration, run the full inference loop, and maintain a lightweight tool-calling agent chain to route user questions to multiple finance analytics agents, ensuring compatibility with CPU, CUDA, and METAL. We allow users to flexibly choose different sizes of LLM (0.5B-7B) according to their own devices (a smaller model may fail for calling tools). Compared to using an API, we can better protect user privacy while reducing money cost, which serves as a practical reference for other similar small projects. 
