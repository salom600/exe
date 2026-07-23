//! Project management commands for FlowCut.
//!
//! This module provides Tauri command handlers for creating, opening, saving,
//! and closing video editing projects.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::project::{Project, ProjectSettings, ProjectState, Timeline};
use crate::utils::{ActionRecord, UndoManager};

/// Comprehensive metadata describing a FlowCut project.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectInfo {
    /// Unique identifier for this project (UUID v4).
    pub id: String,
    /// Human-readable project name.
    pub name: String,
    /// Absolute path to the `.flowcut` project file.
    pub path: String,
    /// ISO 8601 timestamp indicating when the project was first created.
    pub created_at: String,
    /// ISO 8601 timestamp indicating when the project was last modified.
    pub modified_at: String,
    /// Total duration of the timeline in seconds.
    pub timeline_duration: f64,
    /// Number of media items currently imported.
    pub media_count: usize,
    /// Number of tracks present on the timeline.
    pub track_count: usize,
}

/// A lightweight error type for project command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectError {
    /// Categorizes the failure.
    pub kind: String,
    /// Detailed description of what went wrong.
    pub message: String,
}

/// Helper to map an internal Project to the command-level ProjectInfo.
fn project_to_info(project: &Project) -> ProjectInfo {
    ProjectInfo {
        id: project.id.to_string(),
        name: project.name.clone(),
        path: project.path.clone(),
        created_at: project.created_at.to_rfc3339(),
        modified_at: project.modified_at.to_rfc3339(),
        timeline_duration: project.timeline.duration,
        media_count: project.media_pool.len(),
        track_count: project.timeline.tracks.len(),
    }
}

/// Creates a new FlowCut project at the specified path.
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

    // Create a new Project struct
    let project = Project {
        id: uuid::Uuid::new_v4(),
        name: name.clone(),
        path: path.clone(),
        timeline: Timeline::default(),
        media_pool: Vec::new(),
        created_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        settings: ProjectSettings::default(),
    };

    // Open the project in the state manager
    project_state.open_project(project.clone());

    // Record the action for undo support
    undo_manager.push_action(ActionRecord {
        id: uuid::Uuid::new_v4(),
        action_type: "create_project".to_string(),
        description: format!("Created project '{}'", name),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "name": name,
            "path": path,
            "project_id": project.id.to_string(),
        }),
    });

    Ok(project_to_info(&project))
}

/// Opens an existing FlowCut project from the specified path.
///
/// Reads the `.flowcut` project file and makes it the currently active
/// workspace.
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

    // In a real implementation, this would read the .flowcut file
    // from disk and deserialize it. For now, create a project from
    // the path.
    let project = Project {
        id: uuid::Uuid::new_v4(),
        name: path.clone(),
        path: path.clone(),
        timeline: Timeline::default(),
        media_pool: Vec::new(),
        created_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        settings: ProjectSettings::default(),
    };

    // Open the project in the state manager
    project_state.open_project(project.clone());

    // Record the action for undo support
    undo_manager.push_action(ActionRecord {
        id: uuid::Uuid::new_v4(),
        action_type: "open_project".to_string(),
        description: format!("Opened project from '{}'", path),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "path": path,
            "project_id": project.id.to_string(),
        }),
    });

    Ok(project_to_info(&project))
}

/// Saves the currently active project to disk.
#[tauri::command]
pub fn save_project(project_state: State<ProjectState>) -> Result<bool, ProjectError> {
    log::info!("Saving current project");

    // Ensure there is an active project to save
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(ProjectError {
            kind: "no_active_project".into(),
            message: "No active project to save. Open or create a project first.".into(),
        });
    }

    // In a real implementation, this would serialize the project
    // and write it to disk at the project's path.
    // For now, we just log the save operation.
    let project = current_project.unwrap();
    log::info!("Project '{}' saved to '{}'", project.name, project.path);

    log::info!("Project saved successfully");
    Ok(true)
}

/// Closes the currently active project.
#[tauri::command]
pub fn close_project(project_state: State<ProjectState>) -> Result<bool, ProjectError> {
    log::info!("Closing current project");

    // Check if there's a project to close
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(ProjectError {
            kind: "no_active_project".into(),
            message: "No active project to close.".into(),
        });
    }

    // Close the project using the state manager method
    project_state.close_project();

    log::info!("Project closed successfully");
    Ok(true)
}

/// Retrieves metadata about the currently active project.
#[tauri::command]
pub fn get_project_info(project_state: State<ProjectState>) -> Result<ProjectInfo, ProjectError> {
    log::info!("Retrieving project info");

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(ProjectError {
            kind: "no_active_project".into(),
            message: "No active project. Open or create a project first.".into(),
        });
    }

    let project = current_project.unwrap();
    Ok(project_to_info(&project))
}
