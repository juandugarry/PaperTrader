//! Kraken public spot market data. No account, token, real orders or private API.
use crate::crypto::Asset;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, io::Read, time::Duration};
pub const FX_PAIR: &str = "ZAUDZUSD";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    pub session: String,
    pub open_picos: String,
    pub high_picos: String,
    pub low_picos: String,
    pub close_picos: String,
}
#[derive(Debug)]
pub struct Quotes {
    pub aud_usd_picos: i64,
    pub values: Vec<(String, i64)>,
}
fn number(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err("Invalid crypto price.".into()),
    }
}
pub fn decimal(value: &str) -> Result<i64, String> {
    if value.len() > 80 || value.starts_with('-') {
        return Err("Invalid crypto price.".into());
    }
    let (mantissa, exponent) = match value.find(['e', 'E']) {
        Some(i) => (
            &value[..i],
            value[i + 1..]
                .parse::<i32>()
                .map_err(|_| "Invalid crypto price exponent")?,
        ),
        None => (value, 0),
    };
    if !(-30..=30).contains(&exponent) {
        return Err("Crypto price precision outside supported range.".into());
    }
    let decimals = mantissa.split_once('.').map_or(0, |(_, v)| v.len() as i32);
    let digits = mantissa.replace('.', "");
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Invalid crypto price.".into());
    }
    let digits = digits
        .parse::<i128>()
        .map_err(|_| "Crypto price outside supported range")?;
    let scale = 12 + exponent - decimals;
    if !(-30..=30).contains(&scale) {
        return Err("Crypto price precision outside supported range.".into());
    }
    let picos = if scale >= 0 {
        digits
            .checked_mul(10_i128.pow(scale as u32))
            .ok_or("Crypto price overflow")?
    } else {
        let divisor = 10_i128.pow((-scale) as u32);
        digits
            .checked_add(divisor / 2)
            .ok_or("Crypto price rounding overflow")?
            / divisor
    };
    if !(1..=9_000_000_000_000_000_000_i128).contains(&picos) {
        return Err("Crypto price outside supported range (12 decimal places).".into());
    }
    Ok(picos as i64)
}
pub fn to_aud(usd: i64, fx: i64) -> Result<i64, String> {
    if usd <= 0 || fx <= 0 {
        return Err("Missing AUD conversion rate.".into());
    }
    let value = (i128::from(usd) * 1_000_000_000_000 + i128::from(fx) / 2) / i128::from(fx);
    if !(1..=9_000_000_000_000_000_000_i128).contains(&value) {
        return Err("Converted crypto price outside supported range.".into());
    }
    Ok(value as i64)
}
fn result(body: &[u8]) -> Result<Value, String> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| "Invalid Kraken response. Cached crypto data kept.")?;
    let errors = value
        .get("error")
        .and_then(Value::as_array)
        .ok_or("Unexpected Kraken response.")?;
    if !errors.is_empty() {
        return Err("Kraken could not serve this market request. Try again later.".into());
    }
    value
        .get("result")
        .cloned()
        .ok_or("Missing Kraken market data.".into())
}
fn symbol(value: &str) -> String {
    match value {
        "XBT" => "BTC".into(),
        "XDG" => "DOGE".into(),
        _ => value.into(),
    }
}
fn name(value: &str) -> String {
    match value {
        "BTC" => "Bitcoin",
        "ETH" => "Ethereum",
        "SOL" => "Solana",
        "XRP" => "XRP",
        "DOGE" => "Dogecoin",
        "ADA" => "Cardano",
        "LTC" => "Litecoin",
        "BCH" => "Bitcoin Cash",
        "LINK" => "Chainlink",
        "DOT" => "Polkadot",
        "AVAX" => "Avalanche",
        "USDT" => "Tether",
        "USDC" => "USD Coin",
        "TRX" => "TRON",
        _ => value,
    }
    .into()
}
pub fn parse_catalog(body: &[u8]) -> Result<Vec<Asset>, String> {
    let result = result(body)?;
    let rows = result.as_object().ok_or("Invalid Kraken directory.")?;
    if rows.is_empty() || rows.len() > 20000 {
        return Err("Unexpected Kraken directory size.".into());
    }
    let mut assets = BTreeMap::new();
    for (pair, row) in rows {
        let Some(wsname) = row.get("wsname").and_then(Value::as_str) else {
            continue;
        };
        let Some((base, quote)) = wsname.split_once('/') else {
            continue;
        };
        if quote != "USD"
            || [
                "AUD", "USD", "EUR", "GBP", "CAD", "CHF", "JPY", "NZD", "AED", "BRL", "ARS", "TRY",
            ]
            .contains(&base)
            || row
                .get("status")
                .and_then(Value::as_str)
                .is_some_and(|s| s != "online")
        {
            continue;
        }
        if pair.len() > 40
            || !pair
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            || base.len() > 30
            || !base
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        {
            return Err("Invalid Kraken asset identifier.".into());
        }
        let symbol = symbol(base);
        let asset = Asset {
            pair: pair.clone(),
            name: name(&symbol),
            ws_symbol: format!("{symbol}/USD"),
            symbol: symbol.clone(),
            quote: "USD".into(),
        };
        if assets.insert(symbol, asset).is_some() {
            return Err("Ambiguous Kraken crypto market.".into());
        }
    }
    if assets.is_empty() {
        return Err("Kraken returned no active USD crypto markets.".into());
    }
    Ok(assets.into_values().collect())
}
pub fn parse_quotes(body: &[u8], assets: &[Asset]) -> Result<Quotes, String> {
    let result = result(body)?;
    let rows = result
        .as_object()
        .ok_or("Invalid Kraken ticker response.")?;
    let last = |row: &Value| -> Result<i64, String> {
        decimal(&number(
            row.get("c")
                .and_then(|c| c.get(0))
                .ok_or("Missing Kraken closing trade price")?,
        )?)
    };
    let fx = last(
        rows.get(FX_PAIR)
            .or_else(|| rows.get("AUDUSD"))
            .ok_or("Kraken did not supply AUD/USD conversion.")?,
    )?;
    let mut values = vec![];
    for asset in assets {
        if let Some(row) = rows.get(&asset.pair) {
            let price = last(row)?;
            to_aud(price, fx)?;
            values.push((asset.pair.clone(), price));
        }
    }
    if values.is_empty() {
        return Err("No crypto quotes available. Cached values kept.".into());
    }
    Ok(Quotes {
        aud_usd_picos: fx,
        values,
    })
}
pub fn parse_candles(body: &[u8]) -> Result<Vec<Candle>, String> {
    let result = result(body)?;
    let rows = result
        .as_object()
        .ok_or("Invalid Kraken candle response.")?;
    let mut series = rows.iter().filter(|(key, _)| key.as_str() != "last");
    let (_, data) = series.next().ok_or("No crypto history returned")?;
    if series.next().is_some() {
        return Err("Ambiguous crypto candle response.".into());
    }
    let data = data.as_array().ok_or("Invalid crypto history.")?;
    if data.is_empty() || data.len() > 721 {
        return Err("Unexpected crypto history size.".into());
    }
    let mut candles = BTreeMap::new();
    for row in data {
        let row = row.as_array().ok_or("Invalid crypto candle.")?;
        if row.len() < 8 {
            return Err("Incomplete crypto candle.".into());
        }
        let timestamp = row[0].as_i64().ok_or("Invalid crypto candle time")?;
        let date =
            chrono::DateTime::from_timestamp(timestamp, 0).ok_or("Invalid crypto candle time")?;
        if timestamp > chrono::Utc::now().timestamp() + 300 || timestamp < 1_000_000_000 {
            return Err("Invalid crypto candle time.".into());
        }
        let prices = [1, 2, 3, 4].map(|i| number(&row[i]).and_then(|v| decimal(&v)));
        let [o, h, l, c] = prices;
        let (o, h, l, c) = (o?, h?, l?, c?);
        if l > o.min(c) || h < o.max(c) || l > h {
            return Err("Invalid crypto candle range.".into());
        }
        let session = date.to_rfc3339();
        if candles
            .insert(
                session.clone(),
                Candle {
                    session,
                    open_picos: o.to_string(),
                    high_picos: h.to_string(),
                    low_picos: l.to_string(),
                    close_picos: c.to_string(),
                },
            )
            .is_some()
        {
            return Err("Duplicate crypto candle.".into());
        }
    }
    Ok(candles.into_values().collect())
}
fn fetch(endpoint: &str, query: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    static LAST_REQUEST: std::sync::OnceLock<std::sync::Mutex<std::time::Instant>> =
        std::sync::OnceLock::new();
    {
        let mut last = LAST_REQUEST
            .get_or_init(|| {
                std::sync::Mutex::new(std::time::Instant::now() - Duration::from_secs(1))
            })
            .lock()
            .map_err(|_| "Crypto request throttle unavailable")?;
        if let Some(delay) = Duration::from_secs(1).checked_sub(last.elapsed()) {
            std::thread::sleep(delay);
        }
        *last = std::time::Instant::now();
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(25))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialise crypto connection")?;
    let response = client
        .get(format!("https://api.kraken.com/0/public/{endpoint}"))
        .query(query)
        .send()
        .map_err(|_| "Could not reach Kraken. Cached crypto data kept.")?;
    if !response.status().is_success() {
        return Err(if response.status().as_u16() == 429 {
            "Kraken rate limit reached. Wait before refreshing."
        } else {
            "Kraken market data unavailable. Cached values kept."
        }
        .into());
    }
    let mut body = Vec::new();
    response
        .take(10_000_001)
        .read_to_end(&mut body)
        .map_err(|_| "Incomplete Kraken response")?;
    if body.len() > 10_000_000 {
        return Err("Kraken response too large.".into());
    }
    Ok(body)
}
pub fn fetch_catalog() -> Result<Vec<Asset>, String> {
    parse_catalog(&fetch("AssetPairs", &[])?)
}
pub fn fetch_quotes(assets: &[Asset]) -> Result<Quotes, String> {
    parse_quotes(&fetch("Ticker", &[])?, assets)
}
pub fn fetch_candles(pair: &str) -> Result<Vec<Candle>, String> {
    if pair.is_empty()
        || pair.len() > 40
        || !pair
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    {
        return Err("Invalid crypto pair.".into());
    }
    parse_candles(&fetch("OHLC", &[("pair", pair), ("interval", "60")])?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directory_quotes_and_aud_conversion_use_exact_decimal_prices() {
        let assets=parse_catalog(br#"{"error":[],"result":{"XXBTZUSD":{"wsname":"XBT/USD","status":"online"},"XETHZUSD":{"wsname":"ETH/USD"},"ZAUDZUSD":{"wsname":"AUD/USD"},"BTCJPY":{"wsname":"XBT/JPY"}}}"#).unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].symbol, "BTC");
        assert_eq!(assets[0].ws_symbol, "BTC/USD");
        let quotes=parse_quotes(br#"{"error":[],"result":{"XXBTZUSD":{"c":["65000.00000001","1"]},"ZAUDZUSD":{"c":["0.65","1"]}}}"#,&assets).unwrap();
        assert_eq!(
            to_aud(quotes.values[0].1, quotes.aud_usd_picos).unwrap(),
            100_000_000_000_015_385
        );
        assert_eq!(decimal("0.000000000001").unwrap(), 1);
        assert_eq!(decimal("1e-8").unwrap(), 10000);
        assert!(decimal("1e99").is_err());
        assert!(decimal("0").is_err());
        assert!(parse_catalog(br#"{"error":["rate limit"],"result":{}}"#).is_err());
    }
}

#[cfg(test)]
mod public_smoke {
    #[test]
    #[ignore = "Explicit public Kraken network smoke: directory, all quotes and BTC hourly candles; no key"]
    fn public_kraken_catalog_quotes_and_candles() {
        let assets = super::fetch_catalog().unwrap();
        let quotes = super::fetch_quotes(&assets).unwrap();
        let btc = assets.iter().find(|a| a.symbol == "BTC").unwrap();
        let candles = super::fetch_candles(&btc.pair).unwrap();
        assert!(!assets.is_empty());
        assert!(!quotes.values.is_empty());
        assert!(!candles.is_empty());
        println!("Kraken public smoke passed: {} active USD crypto assets, {} quotes, {} hourly BTC candles. No key.",assets.len(),quotes.values.len(),candles.len());
    }
}
