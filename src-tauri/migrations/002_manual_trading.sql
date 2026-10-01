CREATE TABLE securities (
    id INTEGER PRIMARY KEY,
    ticker TEXT NOT NULL UNIQUE CHECK(length(ticker) BETWEEN 1 AND 10 AND ticker NOT GLOB '*[^A-Z0-9]*'),
    name TEXT NOT NULL CHECK(length(trim(name)) BETWEEN 1 AND 120),
    current_price_micros INTEGER CHECK(current_price_micros > 0 AND current_price_micros <= 1000000000000000),
    price_updated_at TEXT,
    CHECK((current_price_micros IS NULL) = (price_updated_at IS NULL))
);
CREATE TABLE executions (
    id INTEGER PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,
    portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
    security_id INTEGER NOT NULL REFERENCES securities(id),
    side TEXT NOT NULL CHECK(side IN ('BUY','SELL')),
    quantity INTEGER NOT NULL CHECK(quantity BETWEEN 1 AND 1000000000),
    price_micros INTEGER NOT NULL CHECK(price_micros BETWEEN 1 AND 1000000000000000),
    brokerage_micros INTEGER NOT NULL CHECK(brokerage_micros BETWEEN 0 AND 1000000000000000 AND brokerage_micros % 10000 = 0),
    notional_micros INTEGER NOT NULL CHECK(notional_micros BETWEEN 10000 AND 1000000000000000 AND notional_micros % 10000 = 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX executions_by_security ON executions(portfolio_id, security_id, id);
CREATE TRIGGER executions_no_update BEFORE UPDATE ON executions
BEGIN SELECT RAISE(ABORT, 'Executions are immutable'); END;
CREATE TRIGGER executions_no_delete BEFORE DELETE ON executions
BEGIN SELECT RAISE(ABORT, 'Executions are immutable'); END;
DROP TRIGGER cash_ledger_no_update;
DROP TRIGGER cash_ledger_no_delete;
ALTER TABLE cash_ledger RENAME TO cash_ledger_v1;
CREATE TABLE cash_ledger (
    id INTEGER PRIMARY KEY,
    portfolio_id INTEGER NOT NULL REFERENCES portfolios(id),
    kind TEXT NOT NULL CHECK(kind IN ('opening_capital','BUY','SELL')),
    amount_micros INTEGER NOT NULL CHECK(abs(amount_micros) <= 1000000000000000 AND amount_micros % 10000 = 0),
    execution_id INTEGER UNIQUE REFERENCES executions(id),
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK((kind = 'opening_capital' AND execution_id IS NULL AND amount_micros > 0)
      OR (kind IN ('BUY','SELL') AND execution_id IS NOT NULL))
);
INSERT INTO cash_ledger(id,portfolio_id,kind,amount_micros,description,created_at)
SELECT id,portfolio_id,kind,amount_micros,description,created_at FROM cash_ledger_v1;
DROP TABLE cash_ledger_v1;
CREATE UNIQUE INDEX one_opening_capital ON cash_ledger(portfolio_id) WHERE kind='opening_capital';
CREATE TRIGGER cash_ledger_validate_execution BEFORE INSERT ON cash_ledger WHEN NEW.execution_id IS NOT NULL
BEGIN
    SELECT CASE WHEN NOT EXISTS (
        SELECT 1 FROM executions e WHERE e.id=NEW.execution_id AND e.portfolio_id=NEW.portfolio_id AND e.side=NEW.kind
        AND NEW.amount_micros = CASE WHEN e.side='BUY' THEN -e.notional_micros-e.brokerage_micros ELSE e.notional_micros-e.brokerage_micros END
    ) THEN RAISE(ABORT, 'Cash movement must match execution') END;
END;
CREATE TRIGGER cash_ledger_no_update BEFORE UPDATE ON cash_ledger
BEGIN SELECT RAISE(ABORT, 'Cash ledger entries are immutable'); END;
CREATE TRIGGER cash_ledger_no_delete BEFORE DELETE ON cash_ledger
BEGIN SELECT RAISE(ABORT, 'Cash ledger entries are immutable'); END;
