-- Existing close-only caches remain valid; refresh retrieves real OHLC candles.
ALTER TABLE daily_prices ADD COLUMN open_micros INTEGER CHECK(open_micros BETWEEN 1 AND 1000000000000000);
ALTER TABLE daily_prices ADD COLUMN high_micros INTEGER CHECK(high_micros BETWEEN 1 AND 1000000000000000);
ALTER TABLE daily_prices ADD COLUMN low_micros INTEGER CHECK(low_micros BETWEEN 1 AND 1000000000000000);
