CREATE TABLE settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) BETWEEN 1 AND 80),
    default_brokerage_micros INTEGER NOT NULL CHECK (default_brokerage_micros >= 0),
    currency TEXT NOT NULL CHECK (currency = 'AUD'),
    primary_market TEXT NOT NULL CHECK (primary_market = 'ASX'),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE portfolios (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE TABLE cash_ledger (
    id INTEGER PRIMARY KEY,
    portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
    kind TEXT NOT NULL CHECK (kind = 'opening_capital'),
    amount_micros INTEGER NOT NULL CHECK (amount_micros > 0),
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (portfolio_id, kind)
);
CREATE TRIGGER cash_ledger_no_update BEFORE UPDATE ON cash_ledger
BEGIN SELECT RAISE(ABORT, 'Cash ledger entries are immutable'); END;
CREATE TRIGGER cash_ledger_no_delete BEFORE DELETE ON cash_ledger
BEGIN SELECT RAISE(ABORT, 'Cash ledger entries are immutable'); END;
