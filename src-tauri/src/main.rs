#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let Some(options) = app_lib::cli::dispatch() else {
        return;
    };
    app_lib::run(options);
}
