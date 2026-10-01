//! Personal, locally cached EODHD active ASX directory. No pricing requests.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, time::Duration};
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub ticker: String,
    pub name: String,
    pub kind: String,
    pub currency: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Directory {
    pub entries: Vec<Listing>,
    pub fetched_at: Option<String>,
}
pub fn parse(body: &[u8]) -> Result<Vec<Listing>, String> {
    let rows: Vec<serde_json::Value> = serde_json::from_slice(body)
        .map_err(|_| "Invalid ASX directory response. Cached directory kept.")?;
    if rows.is_empty() || rows.len() > 20_000 {
        return Err("Unexpected ASX directory size. Cached directory kept.".into());
    }
    let mut entries = BTreeMap::new();
    for row in rows {
        let field = |key: &str, max: usize| -> Result<String, String> {
            let value = row.get(key).and_then(|v| v.as_str()).unwrap_or("").trim();
            if value.is_empty()
                || value.chars().count() > max
                || value.chars().any(char::is_control)
            {
                return Err("Invalid ASX listing. Cached directory kept.".into());
            }
            Ok(value.to_string())
        };
        let ticker = field("Code", 20)?;
        if !ticker
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        {
            return Err("Invalid ASX ticker. Cached directory kept.".into());
        }
        let listing = Listing {
            ticker: ticker.clone(),
            name: field("Name", 500)?,
            kind: field("Type", 80)?,
            currency: field("Currency", 10)?,
        };
        if entries.insert(ticker, listing).is_some() {
            return Err("Duplicate ASX ticker. Cached directory kept.".into());
        }
    }
    Ok(entries.into_values().collect())
}
pub fn fetch(key: &str) -> Result<Vec<Listing>, String> {
    crate::market::validate_key(key)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Unable to initialise directory connection.")?;
    let response = client
        .get("https://eodhd.com/api/exchange-symbol-list/AU")
        .query(&[("api_token", key), ("fmt", "json"), ("delisted", "0")])
        .send()
        .map_err(|_| "Could not reach EODHD. Cached directory kept.")?;
    if !response.status().is_success() {
        return Err(crate::market::response_error(response.status().as_u16()).into());
    }
    let mut body = Vec::new();
    response
        .take(5_000_001)
        .read_to_end(&mut body)
        .map_err(|_| "Incomplete directory response. Cached directory kept.")?;
    if body.len() > 5_000_000 {
        return Err("Directory response too large. Cached directory kept.".into());
    }
    parse(&body)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_names_tickers_types_and_rejects_invalid_catalogs() {
        let row = r#"{"Code":"BHP","Name":"BHP Group","Type":"Common Stock","Currency":"AUD"}"#;
        let entries = parse(format!("[{row}]").as_bytes()).unwrap();
        assert_eq!(entries[0].ticker, "BHP");
        for body in [
            "[]".to_string(),
            "{}".to_string(),
            format!("[{row},{row}]"),
            format!("[{}]", row.replace("BHP\"", "../BHP\"")),
        ] {
            assert!(parse(body.as_bytes()).is_err());
        }
    }
}
