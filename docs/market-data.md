# Phase 4 market-data research

The user agreed to free EODHD end-of-day data after reviewing licensing, cost and trade-offs. Release 0.4.0 integrates explicit refresh and daily charts.

## Verified official documentation (1 October 2026)

Internet access now reaches EODHD and Twelve Data. The earlier EODHD terms URL and Twelve Data ASX URL were obsolete; the corrected sources are below. Marketstack’s pricing page is reachable, but its documentation and terms still returned HTTP 403 tunnel failures.

| Candidate | Documented coverage and limits | Assessment |
| --- | --- | --- |
| EODHD | ASX page lists end-of-day and delayed prices. Historical endpoint documentation says the free plan works for any ticker, with one year of history and 20 calls/day. Each symbol-history request costs one call, including failures. Own key required; demo key is restricted. | Recommended starting point: free end-of-day reference prices and a one-year daily chart for private use, subject to actual ASX key entitlement verification. |
| Twelve Data | Exchange directory lists Australian Securities Exchange, XASX, 20-minute delay, as a data add-on. | Delayed ASX access should not be assumed included in a free plan. Add-on price/entitlement requires further confirmation if selected. |
| Marketstack | Pricing lists free end-of-day data, one year of history and 100 requests/month, non-commercial use. | ASX coverage and detailed terms could not be verified; not recommended over the better-documented candidate yet. |

EODHD’s current pricing page displays the personal historical plan at **€19.99/month** (annual pricing also offered), while its historical endpoint documentation still says “from $19.99/mo”. Treat checkout currency and charges as authoritative; no paid subscription is required for the proposed free prototype. Paid history provides greater depth and request allowances. No paid plan or purchase is authorised or configured.

## Licensing and practical trade-offs

EODHD terms permit non-professional users to store, manipulate and analyse data privately for non-commercial purposes. They prohibit account sharing and redistribution of provider data. The intended design is user-supplied keys, direct local retrieval and private SQLite caching, without shared keys, bundled market datasets or a central data service. Commercial use or sharing retrieved data requires separate licensing. Public source code does not establish permission to redistribute provider data; provider clarification is needed if the app’s usage expands beyond individual private use.

End-of-day means the latest **published completed trading session**, not the latest intraday trade. EODHD says daily prices usually update 2–3 hours after market close. Refresh must show the actual session date and retrieval time, and retain older cached data on errors. A refresh across ten symbols typically uses ten calls, so the free daily allowance is suitable for a small personal portfolio with explicit refresh and caching.

Historical API documentation distinguishes raw OHLC from adjusted close. Use unadjusted closing prices for reference valuations, label chart adjustment explicitly, and never rewrite immutable simulated fills. A user’s ASX API entitlement, ticker mapping and actual responses still need validation after provider agreement and secure key configuration. EODHD also publishes a broad indicative-price disclaimer; do not present its data as guaranteed real-time exchange quotes.

## Official sources

- [EODHD ASX coverage](https://eodhd.com/asx-data)
- [EODHD pricing](https://eodhd.com/pricing)
- [Historical endpoint, free limits, adjustment and update timing](https://eodhd.com/financial-apis/api-for-historical-data-and-volumes)
- [EODHD terms](https://eodhd.com/financial-apis/terms-conditions)
- [Personal versus commercial use](https://eodhd.com/financial-apis/commercial-vs-personal-license-use)
- [Twelve Data exchange directory](https://twelvedata.com/exchanges)
- [Twelve Data pricing](https://twelvedata.com/pricing)
- [Marketstack pricing](https://marketstack.com/product)

## Implemented behaviour (0.4.0)

Rust requests `https://eodhd.com/api/eod/TICKER.AU` with a daily, ascending range covering the last 365 days. The user's API key is stored through macOS Keychain, scoped to the profile generation. IPC reports key presence only; it never returns the stored key. Saving a key does not verify entitlement or spend a request. The first explicit refresh validates actual access through the provider response. Signup opens the fixed official registration URL in the Mac's browser.

No scheduled polling occurs. Each batch reserves one slot per unique selected security before networking, subject to a 20-slot local UTC-day cap and one-minute cooldown. Reservations survive early aborts, failed responses, restarts and profile reset. This conservative counter cannot account for requests made in other apps. A batch has at most 20 securities; concurrent batches are rejected, with a ten-minute recovery lease after a crash. Network timeouts are bounded to 20 seconds per attempted symbol. Authentication, rate-limit and network/service failures stop further requests in that batch; per-symbol entitlement or ticker failures allow the remaining symbols to continue.

Network work runs outside the database mutex. Completion checks profile generation, refresh identifier and each reference-price version. Reset or a concurrent manual-price change prevents stale provider writes. Per-symbol savepoints preserve existing price/history on errors or excessive derived valuations; successful stocks still commit. A response older than the cached provider session is rejected. Keys, request URLs and raw provider error bodies are never returned or logged. HTTPS uses the operating system's trusted certificates, with verification enabled and redirects disabled.

Prices are parsed from JSON decimals directly into integer micro-AUD, rounding extra precision half up. Malformed, duplicate, empty, oversized, future or out-of-range history is rejected. The latest unadjusted close updates valuation; chart adjustment is optional and explicitly labelled. Full cached history is replaced on successful refresh, so adjusted historical revisions are not patched onto stale data. Provider prices never alter executions or commentary. Cached values show their session date and retrieval time and do not claim to be current intraday prices.

Migration 005 preserves existing profiles and manual reference prices. Reset removes prices, charts and the profile's Keychain credential; only anonymous UTC-day request counts remain to protect the allowance. Removing a key alone retains cached prices.

## Validation limits

Cloud tests cover exact parsing, malformed responses, partial failures, valuation rollback, quota/cooldown/concurrency, v4 migration, restart persistence, reset during refresh and concurrent manual-price changes. Frontend tests mock IPC for key setup/removal, refresh outcomes, cached charts, allowance and adjusted-series interaction. A real HTTPS smoke using EODHD's documented public AAPL demo returned 251 daily rows; it verifies provider transport and parsing, not ASX entitlement. That optional network test is ignored in the normal offline test suite and was run explicitly.

No user's key is configured in this cloud environment. A private ASX request, macOS Keychain prompts and native desktop IPC still require validation on the user's Mac. The universal macOS build runs in GitHub Actions; its result cannot be queried here because GitHub API access remains forbidden. No paid subscription or provider dataset is bundled.
