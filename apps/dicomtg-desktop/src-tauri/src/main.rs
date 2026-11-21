#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = dicomtg_desktop_lib::run() {
        eprintln!("DicoMTG failed to start: {error}");
        std::process::exit(1);
    }
}
