//! API keys live in macOS Keychain, never SQLite or frontend persistence.
#[cfg(target_os = "macos")]
fn entry(profile_id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("com.papertrader.desktop.eodhd", profile_id)
        .map_err(|_| "Unable to access macOS Keychain.")
}
#[cfg(target_os = "macos")]
pub fn load(profile_id: &str) -> Result<Option<String>, String> {
    match entry(profile_id)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Unable to read the EODHD key from macOS Keychain. Allow access or unlock your keychain.".into()),
    }
}
#[cfg(target_os = "macos")]
pub fn save(profile_id: &str, key: &str) -> Result<(), String> {
    entry(profile_id)?
        .set_password(key)
        .map_err(|_| "Unable to save the key in macOS Keychain.")
}
#[cfg(target_os = "macos")]
pub fn remove(profile_id: &str) -> Result<(), String> {
    match entry(profile_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(
            "Unable to remove the EODHD key from macOS Keychain. Allow access and try again."
                .into(),
        ),
    }
}
#[cfg(not(target_os = "macos"))]
pub fn load(_: &str) -> Result<Option<String>, String> {
    Ok(None)
}
#[cfg(not(target_os = "macos"))]
pub fn save(_: &str, _: &str) -> Result<(), String> {
    Err("Saved market-data keys currently require macOS Keychain.".into())
}
#[cfg(not(target_os = "macos"))]
pub fn remove(_: &str) -> Result<(), String> {
    Ok(())
}
