mod board;
pub mod cli;
mod commands;
mod config;
mod device_config;

use commands::{AppState, StartupRoute};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(options: cli::LaunchOptions) {
    board::validate_definition(board::BOARD_JSON).expect("embedded board definition must be valid");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .manage(StartupRoute {
            section: if options.open_config {
                Some("Config Studio".to_owned())
            } else {
                None
            },
            started_at: std::time::Instant::now(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::get_workspace,
            commands::scan_ports,
            commands::sync_profile,
            commands::create_profile,
            commands::switch_profile,
            commands::archive_profile,
            commands::save_assignment,
            commands::config_files,
            commands::save_config,
            commands::import_config,
            commands::save_macro,
            commands::delete_macro,
            commands::open_license,
            commands::finish_startup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
