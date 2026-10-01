//! API keys live in the system credential store, never SQLite or frontend persistence.
#[cfg(any(target_os = "macos", target_os = "windows"))]
fn entry(profile_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("com.papertrader.desktop.eodhd", profile_id)
        .map_err(|_| "Unable to access the system credential store.".to_string())
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn load(profile_id: &str) -> Result<Option<String>, String> {
    match entry(profile_id)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Unable to read the EODHD key from the system credential store. Allow access or unlock your credential store.".into()),
    }
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn save(profile_id: &str, key: &str) -> Result<(), String> {
    entry(profile_id)?
        .set_password(key)
        .map_err(|_| "Unable to save the key in the system credential store.".to_string())
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn remove(profile_id: &str) -> Result<(), String> {
    match entry(profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(
            "Unable to remove the EODHD key from the system credential store. Allow access and try again."
                .into(),
        ),
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn load(_: &str) -> Result<Option<String>, String> {
    Ok(None)
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn save(_: &str, _: &str) -> Result<(), String> {
    Err("Saved market-data keys currently require the system credential store.".into())
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn remove(_: &str) -> Result<(), String> {
    Ok(())
}
