mod adapters;
mod application;
mod domain;
mod ports;

use adapters::{
    session_registry::AppState,
    tauri::commands::{
        cancel_agent_run, list_agents, load_goal_file, respond_agent_permission, send_prompt_to_run,
        start_agent_run,
    },
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_agents,
            load_goal_file,
            start_agent_run,
            send_prompt_to_run,
            cancel_agent_run,
            respond_agent_permission
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
