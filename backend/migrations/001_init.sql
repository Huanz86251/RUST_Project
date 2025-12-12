BEGIN;

-- UUID generator
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 1) Users (auth)
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2) Accounts (owned by user)
CREATE TABLE accounts (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  name TEXT NOT NULL,
  account_type TEXT NOT NULL, -- checking, credit, cash...
  currency CHAR(3) NOT NULL DEFAULT 'CAD',
  opening_balance NUMERIC(14,2) NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- Composite key for FK scoping by user_id
  UNIQUE (user_id, id),

  -- Per-user unique account name
  UNIQUE (user_id, name)
);

-- 3) Categories (optional tree via parent_id)
CREATE TABLE categories (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  name TEXT NOT NULL,
  parent_id BIGINT,

  -- Per-user unique category name
  UNIQUE (user_id, name),

  -- Composite key for FK scoping by user_id
  UNIQUE (user_id, id),

  -- Parent must belong to same user
  FOREIGN KEY (user_id, parent_id) REFERENCES categories(user_id, id)
);

-- 4) Transactions (header)
CREATE TABLE transactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  occurred_at DATE NOT NULL,
  payee TEXT,
  memo TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- Composite key for FK scoping by user_id
  UNIQUE (user_id, id)
);

-- 5) Entries (splits)
CREATE TABLE entries (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  tx_id UUID NOT NULL,
  account_id BIGINT NOT NULL,
  category_id BIGINT,

  amount NUMERIC(14,2) NOT NULL, -- +credit, -debit
  note TEXT,

  -- Enforce same-user references
  FOREIGN KEY (user_id, tx_id) REFERENCES transactions(user_id, id) ON DELETE CASCADE,
  FOREIGN KEY (user_id, account_id) REFERENCES accounts(user_id, id),
  FOREIGN KEY (user_id, category_id) REFERENCES categories(user_id, id) ON DELETE SET NULL
);

-- Common indexes
CREATE INDEX entries_user_account_idx ON entries(user_id, account_id);
CREATE INDEX entries_user_tx_idx ON entries(user_id, tx_id);
CREATE INDEX transactions_user_date_idx ON transactions(user_id, occurred_at DESC);
CREATE INDEX accounts_user_idx ON accounts(user_id);
CREATE INDEX categories_user_parent_idx ON categories(user_id, parent_id);

COMMIT;
