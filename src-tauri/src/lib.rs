pub mod domain;
pub mod engine;
pub mod journal;
pub mod store;
pub mod trading;

#[cfg(feature = "desktop")]
mod desktop {
    use super::store::{CreateProfile, Snapshot, Store};
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
        input: CreateSecurity,
        store: tauri::State<'_, Mutex<Store>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .create_security(input)
    }
    #[tauri::command]
    fn set_price(
        input: SetPrice,
        store: tauri::State<'_, Mutex<Store>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .set_price(input)
    }
    #[tauri::command]
    fn execute_trade(
        input: ExecuteTrade,
        store: tauri::State<'_, Mutex<Store>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .execute_trade(input)
    }
    #[tauri::command]
    fn save_journal(
        input: super::journal::SaveJournal,
        store: tauri::State<'_, Mutex<Store>>,
    ) -> Result<Snapshot, String> {
        store
            .lock()
            .map_err(|_| "Database lock unavailable")?
            .save_journal(input)
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
                save_journal
            ])
            .run(tauri::generate_context!())
            .expect("Unable to start PaperTrader");
    }
}
#[cfg(feature = "desktop")]
pub use desktop::run;
