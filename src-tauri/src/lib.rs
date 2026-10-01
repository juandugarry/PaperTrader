pub mod domain;
pub mod store;

#[cfg(feature = "desktop")]
mod desktop {
    use super::store::{CreateProfile, Snapshot, Store};
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
            .invoke_handler(tauri::generate_handler![get_snapshot, create_profile])
            .run(tauri::generate_context!())
            .expect("Unable to start PaperTrader");
    }
}
#[cfg(feature = "desktop")]
pub use desktop::run;
