CREATE TABLE crypto_wallet (
 id INTEGER PRIMARY KEY CHECK(id=1),
 mode TEXT NOT NULL CHECK(mode IN ('separate','shared')),
 request_id TEXT NOT NULL UNIQUE,
 starting_funds_micros INTEGER NOT NULL CHECK(starting_funds_micros BETWEEN 0 AND 1000000000000000),
 created_at TEXT NOT NULL DEFAULT(strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TRIGGER crypto_wallet_no_update BEFORE UPDATE ON crypto_wallet BEGIN SELECT RAISE(ABORT,'Wallet funding mode is fixed after setup'); END;
CREATE TABLE virtual_deposits (
 id INTEGER PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE,
 account TEXT NOT NULL CHECK(account IN ('stocks','crypto')),
 amount_micros INTEGER NOT NULL CHECK(amount_micros BETWEEN 10000 AND 1000000000000000 AND amount_micros%10000=0),
 description TEXT NOT NULL CHECK(length(description)<=200),
 created_at TEXT NOT NULL DEFAULT(strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TRIGGER deposits_no_update BEFORE UPDATE ON virtual_deposits BEGIN SELECT RAISE(ABORT,'Virtual deposits are immutable'); END;
CREATE TRIGGER deposits_no_delete BEFORE DELETE ON virtual_deposits BEGIN SELECT RAISE(ABORT,'Virtual deposits are immutable'); END;
CREATE TABLE crypto_catalog (id INTEGER PRIMARY KEY CHECK(id=1),body TEXT NOT NULL,fetched_at TEXT NOT NULL);
CREATE TABLE crypto_quotes (
 pair TEXT PRIMARY KEY,
 price_picos INTEGER NOT NULL CHECK(price_picos BETWEEN 1 AND 9000000000000000000),
 fetched_at TEXT NOT NULL,
 usd_price_picos INTEGER NOT NULL CHECK(usd_price_picos BETWEEN 1 AND 9000000000000000000),
 source TEXT NOT NULL CHECK(source IN ('rest','live'))
);
CREATE TABLE crypto_candles (pair TEXT PRIMARY KEY,body TEXT NOT NULL,fetched_at TEXT NOT NULL);
CREATE TABLE crypto_executions (
 id INTEGER PRIMARY KEY,
 request_id TEXT NOT NULL UNIQUE,
 pair TEXT NOT NULL,
 symbol TEXT NOT NULL,
 side TEXT NOT NULL CHECK(side IN ('BUY','SELL')),
 quantity_atoms INTEGER NOT NULL CHECK(quantity_atoms BETWEEN 1 AND 9000000000000000000),
 price_picos INTEGER NOT NULL CHECK(price_picos BETWEEN 1 AND 9000000000000000000),
 fee_micros INTEGER NOT NULL CHECK(fee_micros BETWEEN 0 AND 1000000000000000 AND fee_micros%10000=0),
 notional_micros INTEGER NOT NULL CHECK(notional_micros BETWEEN 10000 AND 1000000000000000 AND notional_micros%10000=0),
 cash_delta_micros INTEGER NOT NULL CHECK(abs(cash_delta_micros)<=1000000000000000 AND cash_delta_micros%10000=0),
 notes TEXT NOT NULL CHECK(length(notes)<=10000),
 created_at TEXT NOT NULL DEFAULT(strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TRIGGER crypto_executions_no_update BEFORE UPDATE ON crypto_executions BEGIN SELECT RAISE(ABORT,'Crypto fills are immutable'); END;
CREATE TRIGGER crypto_executions_no_delete BEFORE DELETE ON crypto_executions BEGIN SELECT RAISE(ABORT,'Crypto fills are immutable'); END;
CREATE TABLE crypto_network (
 id INTEGER PRIMARY KEY CHECK(id=1),
 job_id TEXT, started INTEGER,
 aud_usd_picos INTEGER,
 fx_fetched_at TEXT,
 last_error TEXT
);
INSERT INTO crypto_network(id) VALUES(1);
