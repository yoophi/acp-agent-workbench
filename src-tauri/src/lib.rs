mod adapters;
mod application;
mod domain;
mod ports;

use adapters::{
    session_registry::AppState,
    storage_state::StorageState,
    tauri::commands::{
        cancel_agent_run, create_saved_prompt, delete_saved_prompt, list_agents,
        list_saved_prompts, list_workspace_checkouts, list_workspaces, load_goal_file,
        record_saved_prompt_used, refresh_workspace_checkout, register_workspace_from_path,
        remove_workspace, resolve_workspace_workdir, respond_agent_permission, send_prompt_to_run,
        start_agent_run, update_saved_prompt,
    },
};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let storage = tauri::async_runtime::block_on(StorageState::open(app_data_dir))?;
            app.manage(storage);
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_agents,
            load_goal_file,
            start_agent_run,
            send_prompt_to_run,
            cancel_agent_run,
            respond_agent_permission,
            list_workspaces,
            register_workspace_from_path,
            remove_workspace,
            list_workspace_checkouts,
            refresh_workspace_checkout,
            resolve_workspace_workdir,
            list_saved_prompts,
            create_saved_prompt,
            update_saved_prompt,
            delete_saved_prompt,
            record_saved_prompt_used
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
