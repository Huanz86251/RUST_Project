BEGIN;

-- UUID 生成（gen_random_uuid）
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- 1) 使用者账号（注册/登录）
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2) 记账账户（Checking / Credit Card / Cash...）归属用户
CREATE TABLE accounts (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  name TEXT NOT NULL,
  account_type TEXT NOT NULL, -- checking, credit, cash...
  currency CHAR(3) NOT NULL DEFAULT 'CAD',
  opening_balance NUMERIC(14,2) NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- 让 entries 可以用 (user_id, account_id) 做复合外键
  UNIQUE (user_id, id),

  -- 可选：同一用户下账户名唯一（建议开）
  UNIQUE (user_id, name)
);

-- 3) 分类（可做树：parent_id），归属用户
CREATE TABLE categories (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  name TEXT NOT NULL,
  parent_id BIGINT,

  -- 每个用户自己的分类名唯一
  UNIQUE (user_id, name),

  -- 让 entries / parent 引用同一用户的分类
  UNIQUE (user_id, id),

  -- parent 也必须是同一 user 的分类（默认 RESTRICT）
  FOREIGN KEY (user_id, parent_id) REFERENCES categories(user_id, id)
);

-- 4) 交易头（交易信息），归属用户
CREATE TABLE transactions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  occurred_at DATE NOT NULL,
  payee TEXT,
  memo TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

  -- 让 entries 可以用 (user_id, tx_id) 做复合外键
  UNIQUE (user_id, id)
);

-- 5) 分录 entries（一笔交易可以多条 split），归属用户
CREATE TABLE entries (
  id BIGSERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  tx_id UUID NOT NULL,
  account_id BIGINT NOT NULL,
  category_id BIGINT,

  amount NUMERIC(14,2) NOT NULL,  -- positive for credit, negative for debit
  note TEXT,

  -- 强制同一用户体系内引用（关键）
  FOREIGN KEY (user_id, tx_id) REFERENCES transactions(user_id, id) ON DELETE CASCADE,
  FOREIGN KEY (user_id, account_id) REFERENCES accounts(user_id, id),
  FOREIGN KEY (user_id, category_id) REFERENCES categories(user_id, id)
);

-- 常用索引（按 user_id 查询会非常频繁）
CREATE INDEX entries_user_account_idx ON entries(user_id, account_id);
CREATE INDEX entries_user_tx_idx ON entries(user_id, tx_id);
CREATE INDEX transactions_user_date_idx ON transactions(user_id, occurred_at DESC);
CREATE INDEX accounts_user_idx ON accounts(user_id);
CREATE INDEX categories_user_parent_idx ON categories(user_id, parent_id);

COMMIT;
