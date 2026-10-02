//! Public crypto transport and validated local cache updates.
use crate::{
    crypto::{self, Asset},
    crypto_market::{self, Candle, Quotes},
    store::{Snapshot, Store},
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::sync::Mutex;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub pair: String,
    pub price_picos: String,
    pub usd_price_picos: String,
    pub fetched_at: String,
    pub source: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chart {
    pub pair: String,
    pub candles: Vec<Candle>,
    pub fetched_at: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub assets: Vec<Asset>,
    pub quotes: Vec<Quote>,
    pub charts: Vec<Chart>,
    pub catalog_fetched_at: Option<String>,
    pub fx_fetched_at: Option<String>,
    pub last_error: Option<String>,
    pub refreshing: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub market: Market,
    pub snapshot: Snapshot,
}
pub enum Data {
    Catalog(Vec<Asset>, Quotes),
    Quotes(Quotes),
    History(String, Vec<Candle>),
}
pub fn read(c: &Connection) -> Result<Market, String> {
    let assets = crypto::catalog(c)?;
    let fetched: Option<String> = c
        .query_row(
            "SELECT fetched_at FROM crypto_catalog WHERE id=1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let mut q=c.prepare("SELECT pair,price_picos,usd_price_picos,fetched_at,source FROM crypto_quotes ORDER BY pair").map_err(|e|e.to_string())?;
    let quotes = q
        .query_map([], |r| {
            Ok(Quote {
                pair: r.get(0)?,
                price_picos: r.get::<_, i64>(1)?.to_string(),
                usd_price_picos: r.get::<_, i64>(2)?.to_string(),
                fetched_at: r.get(3)?,
                source: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut q = c
        .prepare("SELECT pair,body,fetched_at FROM crypto_candles ORDER BY pair")
        .map_err(|e| e.to_string())?;
    let raw = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let charts = raw
        .into_iter()
        .map(|(pair, body, fetched_at)| {
            Ok(Chart {
                pair,
                candles: serde_json::from_str(&body)
                    .map_err(|_| "Invalid cached crypto candles")?,
                fetched_at,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let (fx_fetched_at,last_error,refreshing)=c.query_row("SELECT fx_fetched_at,last_error,job_id IS NOT NULL AND started>unixepoch()-120 FROM crypto_network WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
    Ok(Market {
        assets,
        quotes,
        charts,
        catalog_fetched_at: fetched,
        fx_fetched_at,
        last_error,
        refreshing,
    })
}
fn store_quotes(c: &Connection, quotes: Quotes) -> Result<(), String> {
    c.execute("UPDATE crypto_network SET aud_usd_picos=?1,fx_fetched_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=1",[quotes.aud_usd_picos]).map_err(|e|e.to_string())?;
    for (pair, usd) in quotes.values {
        let aud = crypto_market::to_aud(usd, quotes.aud_usd_picos)?;
        c.execute("INSERT INTO crypto_quotes(pair,price_picos,usd_price_picos,fetched_at,source) VALUES(?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%fZ','now'),'rest') ON CONFLICT(pair) DO UPDATE SET price_picos=excluded.price_picos,usd_price_picos=excluded.usd_price_picos,fetched_at=excluded.fetched_at,source='rest'",params![pair,aud,usd]).map_err(|e|e.to_string())?;
    }
    Ok(())
}
pub fn apply(c: &Connection, data: Data) -> Result<(), String> {
    match data {
        Data::Catalog(assets, quotes) => {
            let body = serde_json::to_string(&assets).map_err(|e| e.to_string())?;
            c.execute("INSERT INTO crypto_catalog(id,body,fetched_at) VALUES(1,?1,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(id) DO UPDATE SET body=excluded.body,fetched_at=excluded.fetched_at",[body]).map_err(|e|e.to_string())?;
            store_quotes(c, quotes)?;
        }
        Data::Quotes(quotes) => store_quotes(c, quotes)?,
        Data::History(pair, candles) => {
            let body = serde_json::to_string(&candles).map_err(|e| e.to_string())?;
            c.execute("INSERT INTO crypto_candles(pair,body,fetched_at) VALUES(?1,?2,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(pair) DO UPDATE SET body=excluded.body,fetched_at=excluded.fetched_at",params![pair,body]).map_err(|e|e.to_string())?;
        }
    }
    Ok(())
}
pub fn refresh(
    store: &Mutex<Store>,
    profile_id: String,
    action: String,
    pair: Option<String>,
) -> Result<Update, String> {
    let (job, assets) = {
        let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
        let market = db.crypto_market(&profile_id)?;
        if !["catalog", "quotes", "history"].contains(&action.as_str()) {
            return Err("Invalid crypto refresh action.".into());
        }
        if action != "catalog" && market.assets.is_empty() {
            return Err("Load the crypto catalogue first.".into());
        }
        if action == "history" && !market.assets.iter().any(|a| Some(&a.pair) == pair.as_ref()) {
            return Err("Choose a listed crypto asset.".into());
        }
        (db.begin_crypto_refresh(&profile_id)?, market.assets)
    };
    let result = match action.as_str() {
        "catalog" => crypto_market::fetch_catalog().and_then(|assets| {
            crypto_market::fetch_quotes(&assets).map(|quotes| Data::Catalog(assets, quotes))
        }),
        "quotes" => crypto_market::fetch_quotes(&assets).map(Data::Quotes),
        "history" => {
            let pair = pair.ok_or("Missing crypto pair")?;
            crypto_market::fetch_candles(&pair).map(|candles| Data::History(pair, candles))
        }
        _ => unreachable!(),
    };
    let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.finish_crypto_refresh(&profile_id, &job, result)?;
    Ok(Update {
        market: db.crypto_market(&profile_id)?,
        snapshot: db.snapshot()?.ok_or("Profile unavailable")?,
    })
}
pub fn apply_stream(c: &Connection, messages: Vec<String>) -> Result<(), String> {
    if messages.is_empty()
        || messages.len() > 100
        || messages.iter().map(String::len).sum::<usize>() > 2_000_000
    {
        return Err("Crypto stream batch outside supported limits.".into());
    }
    let assets = crypto::catalog(c)?;
    let lookup = assets
        .iter()
        .map(|a| (a.ws_symbol.as_str(), a.pair.as_str()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut values = std::collections::BTreeMap::<String, (i64, String)>::new();
    let mut new_fx = None;
    for body in messages {
        let value: Value = serde_json::from_str(&body).map_err(|_| "Invalid crypto stream data")?;
        if value.get("channel").and_then(Value::as_str) != Some("ticker") {
            return Err("Unexpected crypto stream channel.".into());
        }
        let rows = value
            .get("data")
            .and_then(Value::as_array)
            .ok_or("Missing crypto ticker data")?;
        if rows.len() > 1000 {
            return Err("Crypto stream message too large.".into());
        }
        for row in rows {
            let symbol = row
                .get("symbol")
                .and_then(Value::as_str)
                .ok_or("Missing crypto stream symbol")?;
            let raw = row.get("last").ok_or("Missing crypto stream price")?;
            let text = match raw {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => return Err("Invalid crypto stream price.".into()),
            };
            let price = crypto_market::decimal(&text)?;
            let time = row
                .get("timestamp")
                .and_then(Value::as_str)
                .ok_or("Missing crypto stream timestamp")?;
            let date = chrono::DateTime::parse_from_rfc3339(time)
                .map_err(|_| "Invalid crypto stream timestamp")?;
            if date.timestamp() > chrono::Utc::now().timestamp() + 300
                || date.timestamp() < 1_000_000_000
            {
                return Err("Invalid crypto stream timestamp.".into());
            }
            let timestamp = date
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            if symbol == "AUD/USD" {
                new_fx = Some(price);
                continue;
            }
            let pair = lookup
                .get(symbol)
                .ok_or("Crypto stream symbol is not in the cached catalogue")?;
            values.insert((*pair).to_string(), (price, timestamp));
        }
    }
    if let Some(rate) = new_fx {
        c.execute("UPDATE crypto_network SET aud_usd_picos=?1,fx_fetched_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=1",[rate]).map_err(|e|e.to_string())?;
        let mut q = c
            .prepare("SELECT pair,usd_price_picos FROM crypto_quotes")
            .map_err(|e| e.to_string())?;
        let rows = q
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for (pair, usd) in rows {
            c.execute(
                "UPDATE crypto_quotes SET price_picos=?1 WHERE pair=?2",
                params![crypto_market::to_aud(usd, rate)?, pair],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let rate: Option<i64> = c
        .query_row(
            "SELECT aud_usd_picos FROM crypto_network WHERE id=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let rate = rate.ok_or("Refresh crypto prices to obtain an AUD conversion rate.")?;
    for (pair, (usd, time)) in values {
        let previous: Option<(String, String)> = c
            .query_row(
                "SELECT fetched_at,source FROM crypto_quotes WHERE pair=?1",
                [&pair],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if previous.is_some_and(|(prior, source)| source == "live" && prior > time) {
            continue;
        }
        c.execute("INSERT INTO crypto_quotes(pair,price_picos,usd_price_picos,fetched_at,source) VALUES(?1,?2,?3,?4,'live') ON CONFLICT(pair) DO UPDATE SET price_picos=excluded.price_picos,usd_price_picos=excluded.usd_price_picos,fetched_at=excluded.fetched_at,source='live'",params![pair,crypto_market::to_aud(usd,rate)?,usd,time]).map_err(|e|e.to_string())?;
    }
    Ok(())
}
