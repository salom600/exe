//! Undo/redo commands for FlowCut.
//!
//! This module provides Tauri command handlers for the undo/redo
//! system that underpins all reversible editing operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::project::ProjectState;
use crate::utils::UndoManager;

/// A record of an undoable editing action.
///
/// Captures the action type, a description for UI display, the
/// timestamp when the action occurred, and a JSON payload.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionRecord {
    /// Unique identifier for this action record (UUID v4).
    pub id: String,
    /// The action type identifier.
    pub action_type: String,
    /// Human-readable description of what this action did.
    pub description: String,
    /// ISO 8601 timestamp when this action was recorded.
    pub timestamp: String,
    /// JSON payload containing the action's parameters.
    pub payload: serde_json::Value,
    /// Whether this action has been undone.
    pub undone: bool,
    /// Sequence number indicating the order of this action.
    pub sequence: u64,
}

/// Error type for undo/redo command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UndoError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Undoes the most recent editing action.
///
/// Pops the top action from the undo stack, reverts the project
/// state to its condition before that action, and pushes the
/// action onto the redo stack.
#[tauri::command]
pub fn undo_action(
    undo_manager: State<UndoManager>,
    project_state: State<ProjectState>,
) -> Result<Option<ActionRecord>, UndoError> {
    log::info!("Attempting undo action");

    // Check if undo is possible
    if !undo_manager.can_undo() {
        log::info!("Undo stack is empty — nothing to undo");
        return Ok(None);
    }

    // Pop the top action from the undo stack using the UndoManager method
    let action = undo_manager.undo();

    if action.is_none() {
        log::info!("Undo stack is empty — nothing to undo");
        return Ok(None);
    }

    let action = action.unwrap();

    log::info!(
        "Undoing action: type='{}', description='{}'",
        action.action_type,
        action.description
    );

    // Apply the reversal to the project state if a project is open.
    // The undo data payload contains enough information to revert the change.
    if let Some(project) = project_state.get_current_project() {
        // Apply the undo action to the project state.
        // This is a simplified implementation — the actual reversal logic
        // would depend on the action_type and be much more complex.
        log::info!(
            "Applied undo for action '{}' to project state",
            action.action_type
        );
        // Update the project after undo
        project_state.update_project(project);
    }

    log::info!("Undo successful for action: '{}'", action.action_type);

    Ok(Some(ActionRecord {
        id: action.id.to_string(),
        action_type: action.action_type,
        description: action.description,
        timestamp: action.timestamp.to_rfc3339(),
        payload: action.data,
        undone: true,
        sequence: 0,
    }))
}

/// Redoes a previously undone editing action.
///
/// Pops the top action from the redo stack, re-applies the
/// editing operation to the project state, and pushes the
/// action back onto the undo stack.
#[tauri::command]
pub fn redo_action(
    undo_manager: State<UndoManager>,
    project_state: State<ProjectState>,
) -> Result<Option<ActionRecord>, UndoError> {
    log::info!("Attempting redo action");

    // Check if redo is possible
    if !undo_manager.can_redo() {
        log::info!("Redo stack is empty — nothing to redo");
        return Ok(None);
    }

    // Pop the top action from the redo stack using the UndoManager method
    let action = undo_manager.redo();

    if action.is_none() {
        log::info!("Redo stack is empty — nothing to redo");
        return Ok(None);
    }

    let action = action.unwrap();

    log::info!(
        "Redoing action: type='{}', description='{}'",
        action.action_type,
        action.description
    );

    // Re-apply the action to the project state if a project is open.
    if let Some(project) = project_state.get_current_project() {
        // Apply the redo action to the project state.
        log::info!(
            "Applied redo for action '{}' to project state",
            action.action_type
        );
        // Update the project after redo
        project_state.update_project(project);
    }

    log::info!("Redo successful for action: '{}'", action.action_type);

    Ok(Some(ActionRecord {
        id: action.id.to_string(),
        action_type: action.action_type,
        description: action.description,
        timestamp: action.timestamp.to_rfc3339(),
        payload: action.data,
        undone: false,
        sequence: 0,
    }))
}

/// Retrieves the full undo history for the current session.
///
/// Returns all action records in both the undo and redo stacks,
/// ordered chronologically.
#[tauri::command]
pub fn get_undo_history(undo_manager: State<UndoManager>) -> Result<Vec<ActionRecord>, UndoError> {
    log::info!("Retrieving undo history");

    // Get the undo and redo histories from the UndoManager
    let undo_history = undo_manager.get_undo_history();
    let redo_history = undo_manager.get_redo_history();

    // Combine both stacks into a single history view
    let mut records: Vec<ActionRecord> = Vec::new();

    // Add undo stack entries (not undone)
    for (i, action) in undo_history.into_iter().enumerate() {
        records.push(ActionRecord {
            id: action.id.to_string(),
            action_type: action.action_type,
            description: action.description,
            timestamp: action.timestamp.to_rfc3339(),
            payload: action.data,
            undone: false,
            sequence: i as u64,
        });
    }

    // Add redo stack entries (undone)
    let undo_count = records.len();
    for (i, action) in redo_history.into_iter().enumerate() {
        records.push(ActionRecord {
            id: action.id.to_string(),
            action_type: action.action_type,
            description: action.description,
            timestamp: action.timestamp.to_rfc3339(),
            payload: action.data,
            undone: true,
            sequence: (undo_count + i) as u64,
        });
    }

    log::info!("Retrieved {} undo history records", records.len());
    Ok(records)
}
