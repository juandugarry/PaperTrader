pub mod domain;
pub mod engine;
pub mod journal;
pub mod notes;
pub mod store;
pub mod trading;

#[cfg(feature = "desktop")]
mod desktop {
    use super::store::{CreateProfile, ProfileAction, Snapshot, Store};
    use super::trading::{CreateSecurity, ExecuteTrade, SetPrice};
    use std::sync::Mutex;
    use tauri::Manager;
    #[tauri::command]
    fn get_snapshot(store: tauri::State<'_, Mutex<Store>>) -> Result<Option<Snapshot>, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .snapshot()
    }
    #[tauri::command]
    fn create_profile(
        input: CreateProfile,
        store: tauri::State<'_, Mutex<Store>>,
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
        store: tauri::State<'_, Mutex<Store>>,
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
        store: tauri::State<'_, Mutex<Store>>,
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
        store: tauri::State<'_, Mutex<Store>>,
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
        store: tauri::State<'_, Mutex<Store>>,
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
        store: tauri::State<'_, Mutex<Store>>,
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
        store: tauri::State<'_, Mutex<Store>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .apply_profile_action(&profile_id, ProfileAction::DeleteNote(input))
    }
    #[tauri::command]
    fn reset_profile(
        profile_id: String,
        confirmation: String,
        store: tauri::State<'_, Mutex<Store>>,
    ) -> Result<(), String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .reset_profile(&profile_id, &confirmation)
    }
    pub fn run() {
        tauri::Builder::default()
            .setup(|app| {
                let directory = app.path().app_data_dir()?;
                std::fs::create_dir_all(&directory)?;
                let store = Store::open(directory.join("papertrader.sqlite3"))
                    .map_err(std::io::Error::other)?;
                app.manage(Mutex::new(store));
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
                reset_profile
            ])
            .run(tauri::generate_context!())
            .expect("Unable to start PaperTrader");
    }
}
#[cfg(feature = "desktop")]
pub use desktop::run;
