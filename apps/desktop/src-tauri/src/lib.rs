mod db;

use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(db::Db(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            db::db_open,
            db::db_select,
            db::db_execute,
            db::db_batch,
            db::db_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
