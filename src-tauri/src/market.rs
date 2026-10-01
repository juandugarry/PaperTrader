//! EODHD daily history. No key, request URL or provider error body is exposed to IPC/logs.
use crate::domain::MAX_MONEY;
use chrono::{Days, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use std::{io::Read, time::Duration};

pub const DAILY_LIMIT: i64 = 20;
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DailyPrice {
    pub session_date: String,
    pub close_micros: i64,
    pub adjusted_close_micros: Option<i64>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub security_id: i64,
    pub fetched_at: String,
    pub prices: Vec<DailyPrice>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSnapshot {
    pub requests_today: i64,
    pub daily_limit: i64,
    pub refreshing: bool,
    pub histories: Vec<History>,
}
pub struct RefreshSecurity {
    pub id: i64,
    pub ticker: String,
    pub price_version: i64,
}
pub struct RefreshJob {
    pub profile_id: String,
    pub id: String,
    pub securities: Vec<RefreshSecurity>,
}
pub fn validate_key(key: &str) -> Result<&str, String> {
    let key = key.trim();
    if !(8..=160).contains(&key.len())
        || !key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.')
    {
        return Err("Enter a valid EODHD API key.".into());
    }
    Ok(key)
}
// Parse JSON decimal spelling directly: never use floating point for persisted prices.
fn price(v: &Value) -> Result<i64, String> {
    let raw = v
        .as_number()
        .ok_or("Invalid price in provider response.")?
        .to_string();
    if raw.len() > 32 {
        return Err("Invalid price precision.".into());
    }
    let (mantissa, exponent) = match raw.find(['e', 'E']) {
        Some(i) => (
            &raw[..i],
            raw[i + 1..]
                .parse::<i32>()
                .map_err(|_| "Invalid price exponent.")?,
        ),
        None => (raw.as_str(), 0),
    };
    if mantissa.starts_with('-') || !(-18..=18).contains(&exponent) {
        return Err("Invalid price in provider response.".into());
    }
    let decimals = mantissa.split_once('.').map_or(0, |(_, d)| d.len() as i32);
    let digits = mantissa
        .replace('.', "")
        .parse::<i128>()
        .map_err(|_| "Invalid price in provider response.")?;
    let scale = 6 + exponent - decimals;
    if !(-30..=30).contains(&scale) {
        return Err("Invalid price precision.".into());
    }
    let value = if scale >= 0 {
        digits
            .checked_mul(10_i128.pow(scale as u32))
            .ok_or("Price exceeds limits.")?
    } else {
        let divisor = 10_i128.pow((-scale) as u32);
        (digits + divisor / 2) / divisor
    };
    if value < 1 || value > i128::from(MAX_MONEY) {
        return Err("Provider price is outside supported limits.".into());
    }
    Ok(value as i64)
}
pub fn parse_history(body: &[u8], today: NaiveDate) -> Result<Vec<DailyPrice>, String> {
    let value: Value =
        serde_json::from_slice(body).map_err(|_| "Invalid JSON from EODHD. Cached prices kept.")?;
    let rows = value
        .as_array()
        .ok_or("Unexpected EODHD response. Cached prices kept.")?;
    if rows.is_empty() || rows.len() > 400 {
        return Err("EODHD returned no usable daily history. Cached prices kept.".into());
    }
    let earliest = today
        .checked_sub_days(Days::new(366))
        .ok_or("Invalid date range.")?;
    let mut prices = Vec::with_capacity(rows.len());
    for row in rows {
        let date = row
            .get("date")
            .and_then(Value::as_str)
            .ok_or("Missing session date from EODHD.")?;
        let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| "Invalid EODHD session date.")?;
        if parsed.to_string() != date || parsed > today || parsed < earliest {
            return Err("EODHD session date is outside the requested range.".into());
        }
        let close = price(row.get("close").ok_or("Missing EODHD close.")?)?;
        let adjusted = match row.get("adjusted_close") {
            None | Some(Value::Null) => None,
            Some(v) => Some(price(v)?),
        };
        prices.push(DailyPrice {
            session_date: date.into(),
            close_micros: close,
            adjusted_close_micros: adjusted,
        });
    }
    prices.sort_by(|a, b| a.session_date.cmp(&b.session_date));
    if prices
        .windows(2)
        .any(|w| w[0].session_date == w[1].session_date)
    {
        return Err("Duplicate EODHD session dates. Cached prices kept.".into());
    }
    Ok(prices)
}
fn response_error(status: u16) -> &'static str {
    match status {
        401 => "EODHD rejected the API key. Check it in Settings.",
        403 => "This EODHD key is not entitled to this ASX symbol. Check your account coverage.",
        404 => "ASX ticker not found by EODHD. Check the security's ticker.",
        429 => "EODHD rate limit reached. Wait or check your account allowance.",
        _ => "EODHD is unavailable. Cached prices kept; try again later.",
    }
}
pub fn fetch_history(key: &str, ticker: &str) -> Result<Vec<DailyPrice>, String> {
    let key = validate_key(key)?;
    if ticker.is_empty()
        || ticker.len() > 10
        || !ticker
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err("Invalid ASX ticker.".into());
    }
    fetch_url(&format!("https://eodhd.com/api/eod/{ticker}.AU"), key)
}
fn fetch_url(url: &str, key: &str) -> Result<Vec<DailyPrice>, String> {
    let today = Utc::now().date_naive();
    let from = today
        .checked_sub_days(Days::new(365))
        .ok_or("Invalid date range.")?
        .to_string();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Unable to initialise secure market-data connection.")?;
    let response = client
        .get(url)
        .query(&[
            ("api_token", key),
            ("fmt", "json"),
            ("period", "d"),
            ("order", "a"),
            ("from", &from),
            ("to", &today.to_string()),
        ])
        .send()
        .map_err(|_| "Could not reach EODHD. Cached prices kept; check your connection.")?;
    if !response.status().is_success() {
        return Err(response_error(response.status().as_u16()).into());
    }
    let mut body = Vec::new();
    response
        .take(1_000_001)
        .read_to_end(&mut body)
        .map_err(|_| "Incomplete EODHD response. Cached prices kept.")?;
    if body.len() > 1_000_000 {
        return Err("EODHD response is too large. Cached prices kept.".into());
    }
    parse_history(&body, today)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_exact_closes_adjustment_dates_and_sorts() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let p = parse_history(br#"[{"date":"2026-09-30","close":10.115,"adjusted_close":9.995},{"date":"2026-09-29","close":1.2345675e1}]"#,d).unwrap();
        assert_eq!(p[0].close_micros, 12_345_675);
        assert_eq!(p[1].close_micros, 10_115_000);
        assert_eq!(p[1].adjusted_close_micros, Some(9_995_000));
    }
    #[test]
    fn rejects_corrupt_empty_duplicate_future_and_unbounded_data() {
        let d = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        for b in [
            "[]",
            "{}",
            "not json",
            r#"[{"date":"2026-10-02","close":10}]"#,
            r#"[{"date":"2026-09-30","close":-1}]"#,
            r#"[{"date":"2026-09-30","close":1e99}]"#,
            r#"[{"date":"2026-09-30","close":null}]"#,
            r#"[{"date":"2026-09-30","close":10},{"date":"2026-09-30","close":11}]"#,
        ] {
            assert!(parse_history(b.as_bytes(), d).is_err(), "{b}");
        }
    }
    #[test]
    fn credential_and_errors_do_not_expose_provider_bodies_or_keys() {
        assert!(validate_key("example_key_123").is_ok());
        assert!(validate_key("bad?key&").is_err());
        assert!(response_error(403).contains("entitled"));
        assert!(response_error(429).contains("rate limit"));
    }
    #[test]
    #[ignore = "Explicit network smoke against EODHD's documented public AAPL demo; no private key"]
    fn public_demo_transport_and_history() {
        let prices = fetch_url("https://eodhd.com/api/eod/AAPL.US", "demo").unwrap();
        assert!(!prices.is_empty());
        assert!(prices.iter().all(|p| p.close_micros > 0));
        println!(
            "EODHD HTTPS demo passed: {} daily rows; no user key or ASX entitlement tested.",
            prices.len()
        );
    }
}
