# Phase 4 market-data research status

Automatic ASX quote refresh and historical charts are not integrated yet. The project brief requires discussing provider licensing, cost and trade-offs before selecting a provider. Current plans and rights must be verified before implementation.

On 1 October 2026, this cloud environment returned HTTP 403 tunnel failures for these official research pages:

- https://eodhd.com/pricing
- https://eodhd.com/financial-apis/live-realtime-stocks-api
- https://eodhd.com/terms-conditions
- https://twelvedata.com/exchanges/ASX
- https://marketstack.com/documentation
- https://marketstack.com/product

These are research candidates, not selected or validated providers. No current price, ASX entitlement, delay, historical coverage or redistribution right has been established. The environment configuration draft includes `eodhd.com`, `twelvedata.com` and `marketstack.com`; saving the draft does not apply those network permissions to the running environment. Apply the network settings before resuming verification.

Selection must establish ASX ordinary-share coverage, free versus paid access, end-of-day versus delayed/intraday timestamps, historical adjusted versus unadjusted prices, rate limits, key requirements and permission for personal desktop use. Verify whether distributing this open-source app with user-supplied keys is permitted. Free public visibility does not by itself establish an API licence.

After agreement, implement provider retrieval in Rust, keep keys out of logs and the repository, show source and quote time, preserve stale values on failure, and keep retrieved reference prices separate from immutable simulated fill prices. Historical charts must identify price adjustment and currency. No broker or Google Finance scraping is planned.

Release 0.3.1 delivers the independently requested profile reset and local notepad while this dependency remains unresolved.
