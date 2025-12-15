# Backend Reproducibility Guide for developer
## First clone the git repo, if already cloned, enter backend directionary
git clone https://github.com/Huanz86251/RUST_Project.git
cd RUST_Project/backend
## Then check Rustc version, require at least 1.88
## If not, try to update the rustc.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

rustup update stable
rustup default stable

rustc --version

## Download Docker
### If already downloaded, pls verify

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
  https://download.docker.com/linux/ubuntu \
  $(. /etc/os-release && echo ${UBUNTU_CODENAME:-$VERSION_CODENAME}) stable" | \
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

### Check if Docker daemon is running
sudo systemctl status docker --no-pager
### If not, run
sudo systemctl enable --now docker
### Add user into docker group
sudo usermod -aG docker $USER
newgrp docker
### Now run the docker compose to build the DB.
docker compose up -d

### it should show STATUS healthy
sudo docker compose ps

## If there's no .env file or you want to create your own .env, run these cmd
JWT_SECRET="$(openssl rand -hex 32)" && cat > .env <<EOF
DATABASE_URL=postgres://finance:finance_pw@localhost:5432/finance
JWT_SECRET=$JWT_SECRET
EOF

## Install sqlx
cargo install sqlx-cli --no-default-features --features postgres --locked

## Build the Sql using SQLx migrations
DATABASE_URL=postgres://finance:finance_pw@localhost:5432/finance sqlx migrate run

## Try to run cargo build
cargo sqlx prepare
cargo build

# User guide for backend server without client (Only Curl)

## For developer who want to try server with local database:
cargo run --release
BASE=http://localhost:8080

## For users  or developer that want to try Database stored in a HTTPS back-end server:
BASE=https://finance-backend.bravestone-51d4c984.canadacentral.azurecontainerapps.io
### And you don't need to cargo run.

## 1. Register
curl -i -X POST "$BASE/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test1@example.com",
    "password": "TestPass123!"
  }'

## 2. Login to get JWT – POST /auth/login
## 2.1 Quickly inspect the response
curl -i -X POST "$BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test1@example.com",
    "password": "TestPass123!"
  }'
## 2.2 Use jq to extract the token
TOKEN=$(curl -sS -X POST "$BASE/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test1@example.com",
    "password": "TestPass123!"
  }' | jq -r '.token')

echo "$TOKEN"

## 3. Test root route – GET /
curl -i "$BASE/" \
  -H "Authorization: Bearer $TOKEN"


### Expected something like:

Hello, user_id=xxxx-...

## 4. Accounts – /accounts
## 4.1 Create an account – POST /accounts
curl -i -X POST "$BASE/accounts" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "CIBC Checking",
    "account_type": "checking",
    "currency": "CAD",
    "opening_balance": 1000.00
  }'

## 4.2 List accounts (simple) – GET /accounts
curl -i "$BASE/accounts" \
  -H "Authorization: Bearer $TOKEN"

## 4.3 With balance & filters/sorting
curl -i "$BASE/accounts?include_balance=true&limit=50&offset=0&sort=created_at&order=desc" \
  -H "Authorization: Bearer $TOKEN"


### You can also filter by checking accounts and CAD, for example:

curl -i "$BASE/accounts?type=checking&currency=CAD&include_balance=true" \
  -H "Authorization: Bearer $TOKEN"

## 5. Categories – /categories
## 5.1 Create a top-level category – POST /categories
curl -i -X POST "$BASE/categories" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Food",
    "parent_id": null
  }'

## 5.2 Create a subcategory (assume Food has id = 1)
curl -i -X POST "$BASE/categories" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "Restaurant",
    "parent_id": 1
  }'

## 5.3 List all categories – GET /categories
curl -i "$BASE/categories" \
  -H "Authorization: Bearer $TOKEN"

## 6. Transactions & entries – /transactions

### Your request body structure (CreateTransactionsReq in backend) is:

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

## 6.1 Create a transaction (expense) – POST /transactions

### Example: on account 1, category 2, spend 20.5:

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


### You can also create an “income” transaction:

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

## 6.2 List transactions – GET /transactions

### Simple list (default limit/offset):

curl -i "$BASE/transactions" \
  -H "Authorization: Bearer $TOKEN"


### With pagination parameters:

curl -i "$BASE/transactions?limit=50&offset=0" \
  -H "Authorization: Bearer $TOKEN"


### Each returned TransactionsDto includes entries: Vec<EntriesDto>.

## 7. Ledger summary snapshot – /ledger or /ledger/snapshot

## If you mount the handler on /ledger, it’s roughly like this:

curl -i "$BASE/ledger" \
  -H "Authorization: Bearer $TOKEN"


## If it’s /ledger/snapshot, then:

curl -i "$BASE/ledger/snapshot" \
  -H "Authorization: Bearer $TOKEN"
