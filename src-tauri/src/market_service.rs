//! Blocking orchestration, compiled in headless builds as well as native desktop.
use crate::{
    credentials, market,
    store::{Snapshot, Store},
};
use std::sync::Mutex;
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub configured: bool,
    pub supported: bool,
}
pub fn market_key_status(store: &Mutex<Store>, profile_id: String) -> Result<KeyStatus, String> {
    let db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.verify_profile(&profile_id)?;
    Ok(KeyStatus {
        configured: credentials::load(&profile_id)?.is_some(),
        supported: cfg!(target_os = "macos"),
    })
}
pub fn save_market_key(
    store: &Mutex<Store>,
    profile_id: String,
    key: String,
) -> Result<(), String> {
    let key = market::validate_key(&key)?;
    let db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.verify_profile(&profile_id)?;
    credentials::save(&profile_id, key)
}
pub fn remove_market_key(store: &Mutex<Store>, profile_id: String) -> Result<(), String> {
    let db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.verify_profile(&profile_id)?;
    credentials::remove(&profile_id)
}
pub fn refresh_market_prices(
    store: &Mutex<Store>,
    profile_id: String,
    security_ids: Vec<i64>,
) -> Result<Snapshot, String> {
    let (job, key) = {
        let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
        db.verify_profile(&profile_id)?;
        let key =
            credentials::load(&profile_id)?.ok_or("Add your EODHD API key in Settings first.")?;
        market::validate_key(&key)?;
        (db.begin_refresh(&profile_id, security_ids)?, key)
    };
    // Network requests run without holding the SQLite mutex. Each failed attempt still costs allowance.
    let mut results = Vec::new();
    let mut stop: Option<String> = None;
    for security in &job.securities {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .verify_profile(&job.profile_id)?;
        if let Some(ref message) = stop {
            results.push(Err(message.clone()));
            continue;
        }
        let result = market::fetch_history(&key, &security.ticker);
        if let Err(ref message) = result {
            if message.contains("API key")
                || message.contains("rate limit")
                || message.contains("Could not reach")
                || message.contains("unavailable")
            {
                stop = Some(message.clone());
            }
        }
        results.push(result);
    }
    let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.finish_refresh(job, results)
}
pub fn reset_profile(
    store: &Mutex<Store>,
    profile_id: String,
    confirmation: String,
) -> Result<(), String> {
    let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.verify_profile(&profile_id)?;
    if confirmation != "RESET" {
        return Err("Type RESET to confirm permanently clearing all local progress.".into());
    }
    credentials::remove(&profile_id)?;
    db.reset_profile(&profile_id, &confirmation)
}

#[cfg(target_os = "macos")]
pub fn open_market_signup() -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg("https://eodhd.com/register")
        .status()
        .map_err(|_| "Could not open your browser. Visit eodhd.com to create an account.")?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not open your browser. Visit eodhd.com to create an account.".into())
    }
}
#[cfg(not(target_os = "macos"))]
pub fn open_market_signup() -> Result<(), String> {
    Err("Visit eodhd.com in your browser to create an account.".into())
}
