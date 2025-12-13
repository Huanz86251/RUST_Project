# User guide for backend server without client (Only Curl)
## For developer who want to try run their server with local database:
cargo run

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
