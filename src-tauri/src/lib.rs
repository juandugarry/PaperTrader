mod credentials;
pub mod domain;
pub mod engine;
pub mod journal;
pub mod market;
pub mod market_service;
pub mod notes;
pub mod store;
pub mod trading;

#[cfg(feature = "desktop")]
mod desktop {
    use super::store::{CreateProfile, ProfileAction, Snapshot, Store};
    use super::trading::{CreateSecurity, ExecuteTrade, SetPrice};
    use crate::market_service::{self, KeyStatus};
    use std::sync::{Arc, Mutex};
    use tauri::Manager;
    #[tauri::command]
    fn get_snapshot(
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Option<Snapshot>, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .snapshot()
    }
    #[tauri::command]
    fn create_profile(
        input: CreateProfile,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .create_profile(input)
    }
    #[tauri::command]
    fn create_security(
        profile_id: String,
        input: CreateSecurity,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::CreateSecurity(input))
    }
    #[tauri::command]
    fn set_price(
        profile_id: String,
        input: SetPrice,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::SetPrice(input))
    }
    #[tauri::command]
    fn execute_trade(
        profile_id: String,
        input: ExecuteTrade,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::ExecuteTrade(input))
    }
    #[tauri::command]
    fn save_journal(
        profile_id: String,
        input: super::journal::SaveJournal,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::SaveJournal(input))
    }
    #[tauri::command]
    fn save_note(
        profile_id: String,
        input: super::notes::SaveNote,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::SaveNote(input))
    }
    #[tauri::command]
    fn delete_note(
        profile_id: String,
        input: super::notes::DeleteNote,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::DeleteNote(input))
    }
    #[tauri::command]
    async fn market_key_status(
        profile_id: String,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<KeyStatus, String> {
        let store = store.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            market_service::market_key_status(store.as_ref(), profile_id)
        })
        .await
        .map_err(|_| "Unable to check market-data settings".to_string())?
    }
    #[tauri::command]
    async fn save_market_key(
        profile_id: String,
        key: String,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<(), String> {
        let store = store.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            market_service::save_market_key(store.as_ref(), profile_id, key)
        })
        .await
        .map_err(|_| "Unable to save market-data settings".to_string())?
    }
    #[tauri::command]
    async fn remove_market_key(
        profile_id: String,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<(), String> {
        let store = store.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            market_service::remove_market_key(store.as_ref(), profile_id)
        })
        .await
        .map_err(|_| "Unable to remove market-data settings".to_string())?
    }
    #[tauri::command]
    async fn refresh_market_prices(
        profile_id: String,
        security_ids: Vec<i64>,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<Snapshot, String> {
        let store = store.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            market_service::refresh_market_prices(store.as_ref(), profile_id, security_ids)
        })
        .await
        .map_err(|_| "Price refresh stopped unexpectedly. Cached prices kept.".to_string())?
    }
    #[tauri::command]
    async fn reset_profile(
        profile_id: String,
        confirmation: String,
        store: tauri::State<'_, Arc<Mutex<Store>>>,
    ) -> Result<(), String> {
        let store = store.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            market_service::reset_profile(store.as_ref(), profile_id, confirmation)
        })
        .await
        .map_err(|_| "Unable to reset profile".to_string())?
    }
    #[tauri::command]
    fn open_market_signup() -> Result<(), String> {
        market_service::open_market_signup()
    }
    pub fn run() {
        tauri::Builder::default()
            .setup(|app| {
                let directory = app.path().app_data_dir()?;
                std::fs::create_dir_all(&directory)?;
                let store = Store::open(directory.join("papertrader.sqlite3"))
                    .map_err(std::io::Error::other)?;
                app.manage(Arc::new(Mutex::new(store)));
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                get_snapshot,
                create_profile,
                create_security,
                set_price,
                execute_trade,
                save_journal,
                save_note,
                delete_note,
                reset_profile,
                market_key_status,
                save_market_key,
                remove_market_key,
                refresh_market_prices,
                open_market_signup
            ])
            .run(tauri::generate_context!())
            .expect("Unable to start PaperTrader");
    }
}
#[cfg(feature = "desktop")]
pub use desktop::run;
