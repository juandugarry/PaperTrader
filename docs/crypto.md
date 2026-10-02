# Crypto practice trading · 0.5.0

Crypto uses Kraken’s public spot REST and WebSocket v2 APIs. It does not connect to a personal exchange account, sign a transaction, hold a private key or place an exchange order. All balances and trades are fictional. ASX retrieval remains separate and uses the existing personal EODHD key and allowance.

## Coverage and freshness

The directory includes all active USD spot crypto pairs in the public AssetPairs response, excluding fiat currencies. It normalises XBT to BTC and XDG to DOGE, supplies familiar names for common assets and uses exchange symbols where a full name is unavailable. Coverage is exchange-specific and changes over time; the release smoke test retrieved 626 supported assets and all 626 quotes.

Load/update catalogue retrieves AssetPairs and Ticker. Refresh all prices retrieves Ticker once for all supported assets and AUD/USD. Requests are spaced at least one second apart, explicit batches have a three-second cooldown, and there is no daily crypto app quota. Provider outages and public rate limits still apply. Live updates subscribe to selected and held coins plus AUD/USD through one public WebSocket v2 connection. Subscription changes reuse that connection. Messages are coalesced and saved at most once per second. The connection pauses when Crypto is hidden and retries with five-to-sixty-second backoff.

USD last-trade prices convert to AUD using the latest public AUD/USD rate: USD / AUDUSD. Price labels distinguish retrieval time from live last-trade time, and display conversion retrieval time. A connected feed does not guarantee a recent trade in a thin market. Other catalogue prices remain cached until manually refreshed. Cached data remains available offline; failures retain prior data and show an error. Chart retrieval is explicit and provides up to 721 hourly OHLC rows including the forming candle. Charts show actual USD prices; current FX is not applied to historical prices.

Public references: [REST pairs](https://docs.kraken.com/api/docs/rest-api/get-tradable-asset-pairs/), [REST ticker](https://docs.kraken.com/api/docs/rest-api/get-ticker-information/), [REST OHLC](https://docs.kraken.com/api/docs/rest-api/get-ohlc-data/), [WebSocket ticker](https://docs.kraken.com/api/docs/websocket-v2/ticker/), [REST limits](https://docs.kraken.com/api/docs/guides/spot-rest-ratelimits/), [WebSocket guidance](https://docs.kraken.com/api/docs/guides/spot-ws-intro/). Respect the provider’s terms and limits; no provider data is bundled or redistributed with this repository.

## Funding and accounting

Funding mode is confirmed once per profile. Separate mode starts with the user's chosen fresh fictional AUD funds and has independent cash, contributions and returns. Shared mode spends stock cash and combines stock and crypto asset values exactly once. Shared stock and crypto views use the same contributions and total return. Crypto-specific realised P&L and fees remain separately visible. No funds are automatically imported or transferred.

Virtual deposits are positive whole-cent AUD amounts with a unique request ID and optional note. Stock/shared deposits affect that balance; crypto deposits require a configured separate wallet. Retrying an identical request returns the original result; a conflicting request ID is rejected. Deposits increase contributions, not realised or unrealised gains.

Crypto quantities use integer atoms (100,000,000 per coin). Prices use integer picos (1,000,000,000,000 per AUD). Both cross IPC as decimal strings so JavaScript cannot lose integer precision. Cash uses the existing integer micro-AUD representation. Quantity × price is computed in Rust i128 and rounded half-up to a whole cent. Financial values have the existing A$1 billion bound; quantities and prices also have validated i64-safe bounds. Purchases include virtual fees in cost basis; partial sells proportionally allocate average cost and full closes release the remaining basis. Cash and position checks run in the same SQLite transaction as each immutable fill. Overspending and overselling never write partial history.

Migration 008 adds the wallet, immutable deposits/fills and market caches without rebuilding existing stock ledgers. Existing stock spending checks include shared crypto cash movements. Snapshots subtract all contributions from total value. Profile-generation guards reject stale writes and in-flight network results after reset. Reset recreates the schema and clears crypto configuration, history and caches.

## Validation

The ordinary headless Rust tests cover exact accounting, fractional partial/full closes, immutable and idempotent deposits/fills, shared stock spending, failed cache updates, stale live ticks and reset races. Frontend tests cover funding choices, reviewed trade details staying fixed through price changes, deposits, searches and WebSocket subscription/pause behavior.

An optional explicit trusted-TLS public transport check is available:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --locked --no-default-features public_kraken_catalog_quotes_and_candles -- --ignored --nocapture
```

No API key is required. Ordinary tests use fixtures and make no public requests. For a native desktop smoke test, use a disposable profile, choose a separate A$500 wallet, load the catalogue, select BTC, load the hourly chart, and switch chart styles. Buy 0.001 BTC with a manual A$100,000 price and A$1 fee; cash should be A$399 and quantity 0.001. Deposit A$50; cash becomes A$449, contributions A$550 and investment gain stays unchanged. Stock cash stays independent. Restart and verify wallet, deposits, fills and cached charts persist. Test shared funding on another disposable profile; a crypto purchase must reduce stock cash and preserve combined portfolio value apart from fees. Never reset a portfolio you want to keep.
