//! Project management commands for FlowCut.
//!
//! This module provides Tauri command handlers for creating, opening, saving,
//! and closing video editing projects. Each project encapsulates its timeline,
//! media library, effects configuration, and metadata within a single workspace
//! file that can be persisted to disk.
//!
//! # State Dependencies
//!
//! All commands in this module depend on [`ProjectState`] managed by the Tauri
//! application state system, and [`UndoManager`] for tracking reversible
//! operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use flowcut_lib::project::ProjectState;
use flowcut_lib::utils::UndoManager;

/// Comprehensive metadata describing a FlowCut project.
///
/// This struct is returned by most project-level commands and contains
/// all the information the frontend needs to render the project workspace,
/// including the project name, file path, creation/modification timestamps,
/// timeline duration, and the count of media items in the library.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectInfo {
    /// Unique identifier for this project (UUID v4).
    pub id: String,
    /// Human-readable project name, as specified by the user on creation.
    pub name: String,
    /// Absolute path to the `.flowcut` project file on the filesystem.
    pub path: String,
    /// ISO 8601 timestamp indicating when the project was first created.
    pub created_at: String,
    /// ISO 8601 timestamp indicating when the project was last modified.
    pub modified_at: String,
    /// Total duration of the timeline in seconds, derived from the
    /// furthest-reaching clip on any track.
    pub timeline_duration: f64,
    /// Number of media items currently imported into the project library.
    pub media_count: usize,
    /// Number of tracks present on the timeline (video + audio).
    pub track_count: usize,
}

/// A lightweight error type for project command failures.
///
/// Carries a machine-readable error kind and a human-readable description
/// that the frontend can display to the user.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectError {
    /// Categorizes the failure for programmatic handling on the frontend.
    pub kind: String,
    /// Detailed description of what went wrong.
    pub message: String,
}

/// Creates a new FlowCut project at the specified path.
///
/// This command initializes a fresh project workspace, generates a unique
/// project ID, creates the `.flowcut` directory structure on disk, and
/// registers the project in the application state. After creation, the
/// project becomes the currently active workspace.
///
/// # Parameters
///
/// - `name` — A descriptive name for the project (e.g. "My Vacation Edit").
/// - `path` — The absolute filesystem directory where the project will
///   be stored. The directory must exist and be writable.
///
/// # Returns
///
/// A [`ProjectInfo`] struct populated with the newly created project's
/// metadata, or a [`ProjectError`] if the path is invalid, already contains
/// a project, or filesystem permissions prevent creation.
///
/// # Undo Support
///
/// Creating a project is recorded as an undoable action. Calling
/// [`undo_action`] will revert to the previous project state.
#[tauri::command]
pub fn create_project(
    name: String,
    path: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<ProjectInfo, ProjectError> {
    log::info!("Creating project '{}' at path: {}", name, path);

    // Validate that the name is non-empty and within reasonable length
    if name.trim().is_empty() {
        return Err(ProjectError {
            kind: "validation".into(),
            message: "Project name must not be empty.".into(),
        });
    }
    if name.len() > 256 {
        return Err(ProjectError {
            kind: "validation".into(),
            message: "Project name must not exceed 256 characters.".into(),
        });
    }

    // Validate that the path is provided
    if path.trim().is_empty() {
        return Err(ProjectError {
            kind: "validation".into(),
            message: "Project path must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| ProjectError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    // Delegate to the project module's internal creation logic
    let result = project.create(name.clone(), path.clone()).map_err(|e| {
        ProjectError {
            kind: "creation_failed".into(),
            message: format!("Failed to create project: {}", e),
        }
    })?;

    // Record the action for undo support
    undo_manager
        .record_action("create_project", serde_json::json!({
            "name": name,
            "path": path,
            "project_id": result.id.clone(),
        }))
        .map_err(|e| ProjectError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(ProjectInfo {
        id: result.id,
        name: result.name,
        path: result.path,
        created_at: result.created_at,
        modified_at: result.modified_at,
        timeline_duration: result.timeline_duration,
        media_count: result.media_count,
        track_count: result.track_count,
    })
}

/// Opens an existing FlowCut project from the specified path.
///
/// Reads the `.flowcut` project file, deserializes its contents (timeline,
/// media library, effects, and metadata), and makes it the currently active
/// workspace. Any previously active project is closed before the new one
/// is opened.
///
/// # Parameters
///
/// - `path` — Absolute filesystem path to the `.flowcut` project file.
///
/// # Returns
///
/// A [`ProjectInfo`] struct reflecting the loaded project's current state,
/// or a [`ProjectError`] if the file cannot be found, is corrupted, or
/// is incompatible with this version of FlowCut.
///
/// # Undo Support
///
/// Opening a project is recorded as an undoable action so the user can
/// revert to the previously open project.
#[tauri::command]
pub fn open_project(
    path: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<ProjectInfo, ProjectError> {
    log::info!("Opening project from path: {}", path);

    if path.trim().is_empty() {
        return Err(ProjectError {
            kind: "validation".into(),
            message: "Project path must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| ProjectError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    let result = project.open(path.clone()).map_err(|e| ProjectError {
        kind: "open_failed".into(),
        message: format!("Failed to open project: {}", e),
    })?;

    // Record the action for undo support
    undo_manager
        .record_action("open_project", serde_json::json!({
            "path": path,
            "project_id": result.id.clone(),
        }))
        .map_err(|e| ProjectError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(ProjectInfo {
        id: result.id,
        name: result.name,
        path: result.path,
        created_at: result.created_at,
        modified_at: result.modified_at,
        timeline_duration: result.timeline_duration,
        media_count: result.media_count,
        track_count: result.track_count,
    })
}

/// Saves the currently active project to disk.
///
/// Serializes all project data (timeline, media library, effects, metadata)
/// and writes it to the `.flowcut` file at the project's stored path.
/// This is a full save — not an incremental delta — ensuring the on-disk
/// representation is always a complete and consistent snapshot.
///
/// # Returns
///
/// `true` if the save completed successfully, or a [`ProjectError`] if
/// there is no active project, the project path is invalid, or a
/// filesystem I/O error occurs.
///
/// # Undo Support
///
/// Saving is not recorded as an undoable action since it is a persistence
/// operation that does not alter the logical state of the project.
#[tauri::command]
pub fn save_project(
    project_state: State<ProjectState>,
) -> Result<bool, ProjectError> {
    log::info!("Saving current project");

    let mut project = project_state.data.lock().map_err(|e| ProjectError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    // Ensure there is an active project to save
    if !project.is_open() {
        return Err(ProjectError {
            kind: "no_active_project".into(),
            message: "No active project to save. Open or create a project first.".into(),
        });
    }

    project.save().map_err(|e| ProjectError {
        kind: "save_failed".into(),
        message: format!("Failed to save project: {}", e),
    })?;

    log::info!("Project saved successfully");
    Ok(true)
}

/// Closes the currently active project.
///
/// Unloads the project from the application state, releasing all associated
/// resources (timeline data, media references, effect configurations).
/// If the project has unsaved changes, this command does **not** auto-save;
/// the frontend should prompt the user to save before calling this command.
///
/// # Returns
///
/// `true` if the project was closed successfully, or a [`ProjectError`] if
/// there is no active project to close.
///
/// # Undo Support
///
/// Closing is not recorded as an undoable action since it clears the
/// workspace state. The frontend should handle re-opening via
/// [`open_project`] if the user wishes to restore.
#[tauri::command]
pub fn close_project(
    project_state: State<ProjectState>,
) -> Result<bool, ProjectError> {
    log::info!("Closing current project");

    let mut project = project_state.data.lock().map_err(|e| ProjectError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(ProjectError {
            kind: "no_active_project".into(),
            message: "No active project to close.".into(),
        });
    }

    project.close().map_err(|e| ProjectError {
        kind: "close_failed".into(),
        message: format!("Failed to close project: {}", e),
    })?;

    log::info!("Project closed successfully");
    Ok(true)
}

/// Retrieves metadata about the currently active project.
///
/// Returns a snapshot of the project's current state without modifying
/// anything. This is useful for the frontend to refresh its workspace
/// header, display the project name, or check modification timestamps.
///
/// # Returns
///
/// A [`ProjectInfo`] struct reflecting the current project, or a
/// [`ProjectError`] if no project is currently active.
#[tauri::command]
pub fn get_project_info(
    project_state: State<ProjectState>,
) -> Result<ProjectInfo, ProjectError> {
    log::info!("Retrieving project info");

    let project = project_state.data.lock().map_err(|e| ProjectError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(ProjectError {
            kind: "no_active_project".into(),
            message: "No active project. Open or create a project first.".into(),
        });
    }

    let info = project.get_info().map_err(|e| ProjectError {
        kind: "info_failed".into(),
        message: format!("Failed to retrieve project info: {}", e),
    })?;

    Ok(ProjectInfo {
        id: info.id,
        name: info.name,
        path: info.path,
        created_at: info.created_at,
        modified_at: info.modified_at,
        timeline_duration: info.timeline_duration,
        media_count: info.media_count,
        track_count: info.track_count,
    })
}
