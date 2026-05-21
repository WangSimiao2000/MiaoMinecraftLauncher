#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_instances,
            commands::get_versions,
            commands::create_instance,
            commands::delete_instance,
            commands::launch_instance,
            commands::search_mods,
            commands::install_mod,
            commands::get_mods,
            commands::toggle_mod,
            commands::delete_mod,
            commands::export_mrpack,
            commands::import_mrpack,
            commands::get_config,
            commands::save_config,
            commands::detect_java,
            commands::download_java,
            commands::add_offline_account,
        ])
        .run(tauri::generate_context!())
        .expect("error running MiaoMC");
}
