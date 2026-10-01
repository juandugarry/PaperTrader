# Phase 4 market-data research

Automatic ASX quote refresh and historical charts are not integrated yet. The project brief requires discussing licensing, cost and trade-offs before choosing a provider.

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

## Next step

Await agreement to use free EODHD end-of-day data versus investigating paid delayed pricing. Then implement Rust retrieval, secure local key handling, timestamped cached reference prices, explicit refresh, rate-limit/error handling and historical charts. Do not put keys in chat, logs or the repository. No provider client or credential requirement has been configured yet.

Release 0.3.1 already delivers profile reset and local notepad. This research update does not change application behaviour.
