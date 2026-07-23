//! Undo/redo commands for FlowCut.
//!
//! This module provides Tauri command handlers for the undo/redo
//! system that underpins all reversible editing operations. Every
//! destructive command (clip addition/removal, filter application,
//! media import, etc.) records an [`ActionRecord`] in the undo
//! history stack. The user can step backward through the history
//! to undo actions, or step forward to redo previously undone
//! actions.
//!
//! # Undo Architecture
//!
//! The undo system operates as a bidirectional stack:
//! - **Undo stack** — actions that can be undone (moving backward).
//! - **Redo stack** — actions that were undone and can be re-applied
//!   (moving forward).
//!
//! When a new action is recorded, the redo stack is cleared (branching
//! the history). This follows the standard linear undo model used by
//! most editing applications.
//!
//! # State Dependencies
//!
//! Commands depend on [`UndoManager`] for the undo history stack
//! and [`ProjectState`] for applying/reverting the actual state
//! changes.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::project::ProjectState;
use crate::utils::UndoManager;

/// A record of an undoable editing action.
///
/// Captures the action type, a description for UI display, the
/// timestamp when the action occurred, and a JSON payload
/// containing all the information needed to either revert or
/// re-apply the action.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionRecord {
    /// Unique identifier for this action record (UUID v4).
    pub id: String,
    /// The action type identifier (e.g. "add_clip_to_track",
    /// "remove_media", "apply_filter", "move_clip", "split_clip",
    /// "trim_clip", "add_track", "remove_track", "add_transition",
    /// "remove_transition", "create_project", "open_project",
    /// "import_media", "remove_media", "update_filter_params").
    pub action_type: String,
    /// Human-readable description of what this action did.
    /// Displayed in the undo history panel and in tooltips
    /// over undo/redo buttons.
    pub description: String,
    /// ISO 8601 timestamp when this action was recorded.
    pub timestamp: String,
    /// JSON payload containing the action's parameters and any
    /// state snapshots needed for reversal. The schema varies
    /// per action type; see each command's documentation for
    /// the specific JSON structure recorded.
    pub payload: serde_json::Value,
    /// Whether this action has been undone (i.e. it is currently
    /// on the redo stack rather than the undo stack).
    pub undone: bool,
    /// Sequence number indicating the order of this action
    /// relative to others in the history. 0 is the first action.
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
/// action onto the redo stack so it can be re-applied later.
///
/// # Returns
///
/// The [`ActionRecord`] that was undone, providing the frontend
/// with information about what was reverted for display in a
/// notification or status bar message. Returns `None` (as a
/// [`UndoError`] with kind "empty_stack") if the undo stack
/// is empty (nothing to undo).
///
/// # State Changes
///
/// After an undo, the project state is modified to reflect the
/// reversion. The frontend should call [`get_timeline_state`]
/// or other state-query commands to refresh its views.
///
/// # Limitations
///
/// Some actions may not be fully reversible (e.g. if the source
/// media file has been deleted from disk since the import action
/// was recorded). In such cases, the undo operation will revert
/// as much state as possible and return a partial success with
/// a warning in the [`ActionRecord::description`].
#[tauri::command]
pub fn undo_action(
    undo_manager: State<UndoManager>,
    project_state: State<ProjectState>,
) -> Result<Option<ActionRecord>, UndoError> {
    log::info!("Attempting undo action");

    let mut undo = undo_manager.data.lock().map_err(|e| UndoError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire undo manager lock: {}", e),
    })?;

    let mut project = project_state.data.lock().map_err(|e| UndoError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    // Check if the undo stack is empty
    if undo.is_undo_stack_empty() {
        log::info!("Undo stack is empty — nothing to undo");
        return Ok(None);
    }

    // Pop the top action from the undo stack
    let action = undo.pop_undo().map_err(|e| UndoError {
        kind: "pop_failed".into(),
        message: format!("Failed to pop action from undo stack: {}", e),
    })?;

    log::info!(
        "Undoing action: type='{}', description='{}'",
        action.action_type,
        action.description
    );

    // Apply the reversal to the project state
    project.apply_undo(&action).map_err(|e| UndoError {
        kind: "undo_apply_failed".into(),
        message: format!(
            "Failed to apply undo for action '{}': {}",
            action.action_type, e
        ),
    })?;

    // Push the undone action onto the redo stack
    undo.push_redo(action.clone()).map_err(|e| UndoError {
        kind: "redo_push_failed".into(),
        message: format!("Failed to push action to redo stack: {}", e),
    })?;

    log::info!("Undo successful for action: '{}'", action.action_type);

    Ok(Some(ActionRecord {
        id: action.id,
        action_type: action.action_type,
        description: action.description,
        timestamp: action.timestamp,
        payload: action.payload,
        undone: true,
        sequence: action.sequence,
    }))
}

/// Redoes a previously undone editing action.
///
/// Pops the top action from the redo stack, re-applies the
/// editing operation to the project state, and pushes the
/// action back onto the undo stack so it can be undone again.
///
/// # Returns
///
/// The [`ActionRecord`] that was re-applied, providing the
/// frontend with information about what was restored. Returns
/// `None` (as a [`UndoError`] with kind "empty_stack") if the
/// redo stack is empty (nothing to redo).
///
/// # State Changes
///
/// After a redo, the project state is modified to reflect the
/// re-applied action. The frontend should call state-query
/// commands to refresh its views.
///
/// # Prerequisites
///
/// Redo is only available after at least one undo has been
/// performed and no new actions have been recorded since.
/// Recording a new action clears the redo stack.
#[tauri::command]
pub fn redo_action(
    undo_manager: State<UndoManager>,
    project_state: State<ProjectState>,
) -> Result<Option<ActionRecord>, UndoError> {
    log::info!("Attempting redo action");

    let mut undo = undo_manager.data.lock().map_err(|e| UndoError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire undo manager lock: {}", e),
    })?;

    let mut project = project_state.data.lock().map_err(|e| UndoError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    // Check if the redo stack is empty
    if undo.is_redo_stack_empty() {
        log::info!("Redo stack is empty — nothing to redo");
        return Ok(None);
    }

    // Pop the top action from the redo stack
    let action = undo.pop_redo().map_err(|e| UndoError {
        kind: "pop_failed".into(),
        message: format!("Failed to pop action from redo stack: {}", e),
    })?;

    log::info!(
        "Redoing action: type='{}', description='{}'",
        action.action_type,
        action.description
    );

    // Re-apply the action to the project state
    project.apply_redo(&action).map_err(|e| UndoError {
        kind: "redo_apply_failed".into(),
        message: format!(
            "Failed to apply redo for action '{}': {}",
            action.action_type, e
        ),
    })?;

    // Push the re-applied action back onto the undo stack
    undo.push_undo(action.clone()).map_err(|e| UndoError {
        kind: "undo_push_failed".into(),
        message: format!("Failed to push action to undo stack: {}", e),
    })?;

    log::info!("Redo successful for action: '{}'", action.action_type);

    Ok(Some(ActionRecord {
        id: action.id,
        action_type: action.action_type,
        description: action.description,
        timestamp: action.timestamp,
        payload: action.payload,
        undone: false,
        sequence: action.sequence,
    }))
}

/// Retrieves the full undo history for the current session.
///
/// Returns all action records in both the undo and redo stacks,
/// ordered chronologically. The frontend can use this to display
/// an undo history panel that allows the user to jump to any
/// point in the editing timeline (multi-step undo).
///
/// # Returns
///
/// A vector of [`ActionRecord`] structs representing the complete
/// editing history. Actions on the undo stack have `undone: false`,
/// and actions on the redo stack have `undone: true`. The records
/// are sorted by `sequence` number, providing a linear timeline
/// of all editing operations.
///
/// # Note
///
/// This command is read-only and does not modify the undo/redo
/// stacks or the project state.
#[tauri::command]
pub fn get_undo_history(undo_manager: State<UndoManager>) -> Result<Vec<ActionRecord>, UndoError> {
    log::info!("Retrieving undo history");

    let undo = undo_manager.data.lock().map_err(|e| UndoError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire undo manager lock: {}", e),
    })?;

    let history = undo.get_full_history().map_err(|e| UndoError {
        kind: "history_query_failed".into(),
        message: format!("Failed to retrieve undo history: {}", e),
    })?;

    let records = history
        .into_iter()
        .map(|action| ActionRecord {
            id: action.id,
            action_type: action.action_type,
            description: action.description,
            timestamp: action.timestamp,
            payload: action.payload,
            undone: action.undone,
            sequence: action.sequence,
        })
        .collect();

    log::info!("Retrieved {} undo history records", records.len());
    Ok(records)
}
