pub mod commands;
pub mod engine;
pub mod project;
pub mod export;
pub mod utils;

use tauri::Manager;

/// Initialize and run the FlowCut application
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::project::create_project,
            commands::project::open_project,
            commands::project::save_project,
            commands::project::close_project,
            commands::project::get_project_info,
            commands::media::import_media,
            commands::media::get_media_info,
            commands::media::remove_media,
            commands::media::list_media,
            commands::timeline::add_clip_to_track,
            commands::timeline::remove_clip_from_track,
            commands::timeline::move_clip,
            commands::timeline::split_clip,
            commands::timeline::trim_clip,
            commands::timeline::get_timeline_state,
            commands::timeline::add_track,
            commands::timeline::remove_track,
            commands::timeline::add_transition,
            commands::timeline::remove_transition,
            commands::preview::render_preview_frame,
            commands::preview::get_preview_info,
            commands::preview::seek_preview,
            commands::effects::apply_filter,
            commands::effects::remove_filter,
            commands::effects::list_filters,
            commands::effects::get_filter_params,
            commands::effects::update_filter_params,
            commands::export::start_export,
            commands::export::get_export_progress,
            commands::export::cancel_export,
            commands::export::get_export_formats,
            commands::engine::initialize_engine,
            commands::engine::get_engine_status,
            commands::engine::get_system_info,
            commands::keyboard::get_shortcuts,
            commands::keyboard::set_shortcut,
            commands::undo::undo_action,
            commands::undo::redo_action,
            commands::undo::get_undo_history,
        ])
        .setup(|app| {
            // Initialize the video processing engine
            let engine_state = engine::EngineState::new();
            app.manage(engine_state);

            // Initialize the project state
            let project_state = project::ProjectState::new();
            app.manage(project_state);

            // Initialize the export state
            let export_state = export::ExportState::new();
            app.manage(export_state);

            // Initialize the undo system
            let undo_state = utils::UndoManager::new();
            app.manage(undo_state);

            log::info!("FlowCut engine initialized successfully");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running FlowCut");
}
