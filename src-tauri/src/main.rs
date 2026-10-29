#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}\n", info);
        let _ = std::fs::write("D:\\EMTYPYIE\\Kohlrausch\\panic.log", msg);
    }));

    let Some(options) = app_lib::cli::dispatch() else {
        return;
    };
    app_lib::run(options);
}
