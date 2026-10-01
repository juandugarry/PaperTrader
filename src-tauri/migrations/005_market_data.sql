ALTER TABLE securities ADD COLUMN price_source TEXT NOT NULL DEFAULT 'manual' CHECK(price_source IN ('manual','eodhd'));
ALTER TABLE securities ADD COLUMN price_as_of TEXT;
ALTER TABLE securities ADD COLUMN price_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE securities ADD COLUMN market_fetched_at TEXT;
ALTER TABLE securities ADD COLUMN market_error TEXT;
ALTER TABLE settings ADD COLUMN market_refresh_id TEXT;
ALTER TABLE settings ADD COLUMN market_refresh_started INTEGER;
CREATE TABLE daily_prices (
    security_id INTEGER NOT NULL REFERENCES securities(id),
    session_date TEXT NOT NULL,
    close_micros INTEGER NOT NULL CHECK(close_micros BETWEEN 1 AND 1000000000000000),
    adjusted_close_micros INTEGER CHECK(adjusted_close_micros BETWEEN 1 AND 1000000000000000),
    PRIMARY KEY(security_id, session_date)
);
-- Request allowances survive profile reset, so reset cannot bypass the local daily cap.
-- This table contains only UTC day and usage count, no profile or market data.
CREATE TABLE IF NOT EXISTS market_usage (
    utc_day TEXT PRIMARY KEY,
    requests INTEGER NOT NULL CHECK(requests BETWEEN 0 AND 20)
);
