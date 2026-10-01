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
    pub storage: &'static str,
}
pub fn market_key_status(store: &Mutex<Store>, profile_id: String) -> Result<KeyStatus, String> {
    let db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.verify_profile(&profile_id)?;
    Ok(KeyStatus {
        configured: credentials::configured(&db.credential_path(&profile_id)?)?,
        supported: true,
        storage: "Local file on this device",
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
    credentials::save(&db.credential_path(&profile_id)?, key)
}
pub fn remove_market_key(store: &Mutex<Store>, profile_id: String) -> Result<(), String> {
    let db = store.lock().map_err(|_| "Database lock unavailable")?;
    db.verify_profile(&profile_id)?;
    credentials::remove(&db.credential_path(&profile_id)?)
}
pub fn refresh_market_prices(
    store: &Mutex<Store>,
    profile_id: String,
    security_ids: Vec<i64>,
) -> Result<Snapshot, String> {
    let (job, key) = {
        let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
        db.verify_profile(&profile_id)?;
        let key = credentials::load(&db.credential_path(&profile_id)?)?
            .ok_or("Add your EODHD API key in Settings first.")?;
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
    credentials::remove(&db.credential_path(&profile_id)?)?;
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
#[cfg(target_os = "windows")]
pub fn open_market_signup() -> Result<(), String> {
    let status = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", "https://eodhd.com/register"])
        .status()
        .map_err(|_| "Could not open browser. Visit eodhd.com to create an account.")?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not open browser. Visit eodhd.com to create an account.".into())
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn open_market_signup() -> Result<(), String> {
    Err("Visit eodhd.com in your browser to create an account.".into())
}

pub fn refresh_asx_directory(
    store: &Mutex<Store>,
    profile_id: String,
) -> Result<crate::directory::Directory, String> {
    let (job, key) = {
        let mut db = store.lock().map_err(|_| "Database lock unavailable")?;
        db.verify_profile(&profile_id)?;
        let key = credentials::load(&db.credential_path(&profile_id)?)?
            .ok_or("Add your EODHD API key in Settings first.")?;
        market::validate_key(&key)?;
        (db.begin_directory(&profile_id)?, key)
    };
    let result = crate::directory::fetch(&key);
    store
        .lock()
        .map_err(|_| "Database lock unavailable")?
        .finish_directory(&profile_id, &job, result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn key_status_does_not_read_key_contents_and_reset_removes_local_file() {
        let directory =
            std::env::temp_dir().join(format!("papertrader-key-service-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let db = Mutex::new(Store::open(directory.join("portfolio.sqlite3")).unwrap());
        let id = db
            .lock()
            .unwrap()
            .create_profile(crate::store::CreateProfile {
                display_name: "Test".into(),
                starting_capital_micros: 1_000_000_000,
                default_brokerage_micros: 3_000_000,
            })
            .unwrap()
            .profile_id;
        assert!(!market_key_status(&db, id.clone()).unwrap().configured);
        save_market_key(&db, id.clone(), "example_key_123".into()).unwrap();
        let path = db
            .lock()
            .unwrap()
            .credential_path(&id)
            .unwrap()
            .to_path_buf();
        std::fs::write(&path, "invalid?key").unwrap();
        assert!(market_key_status(&db, id.clone()).unwrap().configured);
        assert!(refresh_market_prices(&db, id.clone(), vec![1]).is_err());
        assert_eq!(
            db.lock()
                .unwrap()
                .snapshot()
                .unwrap()
                .unwrap()
                .market
                .requests_today,
            0
        );
        save_market_key(&db, id.clone(), "example_key_123".into()).unwrap();
        assert!(
            save_market_key(&db, "stale-profile".into(), "replacement_key_456".into()).is_err()
        );
        reset_profile(&db, id, "RESET".into()).unwrap();
        assert!(!path.exists());
        drop(db);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
