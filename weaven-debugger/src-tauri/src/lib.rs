//! Weaven Debugger — Tauri 2.x backend.
//!
//! Business logic lives in `weaven-debugger-core`.
//! This crate provides the Tauri command layer.

pub mod commands;

use commands::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            session: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_schema,
            commands::tick,
            commands::tick_n,
            commands::seek_tick,
            commands::get_topology,
            commands::get_cascade_steps,
            commands::inject_signal,
        ])
        .run(tauri::generate_context!())
        .expect("error running weaven-debugger");
}
