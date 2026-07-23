//! Keyboard shortcut commands for FlowCut.
//!
//! This module provides Tauri command handlers for managing the
//! keyboard shortcut system. FlowCut allows users to customize
//! key bindings for common editing actions (e.g. play/pause,
//! split clip, undo, export). Shortcuts are persisted as part
//! of the project settings and can be queried or modified at
//! any time.
//!
//! # Shortcut System
//!
//! Each shortcut maps an **action** (a named command identifier)
//! to a **key** + **modifiers** combination. Modifiers include
//! standard platform-specific keys (Ctrl/Cmd, Alt/Option, Shift,
//! Super/Meta). The frontend registers these shortcuts with the
//! keyboard event system and translates matches into Tauri
//! command invocations.
//!
//! # State Dependencies
//!
//! Commands depend on [`ProjectState`] for shortcut persistence
//! within project settings.

use serde::{Deserialize, Serialize};
use tauri::State;

use flowcut_lib::project::ProjectState;

/// Describes a single keyboard shortcut mapping.
///
/// Associates an action identifier with the key combination
/// that triggers it. The frontend uses this to register global
/// and context-sensitive keyboard listeners.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShortcutDefinition {
    /// The action identifier this shortcut triggers.
    /// Examples: "play_pause", "split_clip", "undo", "redo",
    /// "save_project", "zoom_in", "zoom_out", "add_marker",
    /// "toggle_mute", "delete_selection", "copy", "paste",
    /// "select_all", "export", "seek_start", "seek_end".
    pub action: String,
    /// The primary key that triggers the action.
    /// Uses standard key names: single characters ("a", "s"),
    /// special keys ("Space", "Enter", "Escape", "Delete",
    /// "Backspace", "ArrowUp", "ArrowDown", "ArrowLeft",
    /// "ArrowRight", "Home", "End", "PageUp", "PageDown",
    /// "F1"–"F12", "Tab", "Insert").
    pub key: String,
    /// Modifier keys required alongside the primary key.
    /// Each modifier is a string: "ctrl", "alt", "shift",
    /// "meta" (Cmd on macOS, Win on Windows, Super on Linux).
    pub modifiers: Vec<String>,
    /// Human-readable description of what this action does.
    /// Used in the shortcuts settings panel for clarity.
    pub description: String,
    /// The context in which this shortcut is active.
    /// "global" — always active.
    /// "timeline" — active only when the timeline is focused.
    /// "preview" — active only when the preview viewport is focused.
    /// "media_browser" — active only in the media browser panel.
    pub context: String,
    /// Whether this shortcut can be overridden by the user.
    /// Some core shortcuts (e.g. "save_project") may be locked
    /// to prevent accidental unbinding.
    pub customizable: bool,
    /// The default key binding for this action, before any
    /// user customization. Useful for displaying in the UI
    /// when the user wants to reset a shortcut to its default.
    pub default_key: String,
    /// The default modifiers for this action.
    pub default_modifiers: Vec<String>,
}

/// Error type for keyboard shortcut command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShortcutError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Retrieves all keyboard shortcut definitions.
///
/// Returns the complete set of shortcuts, including both default
/// bindings and any user-customized overrides. The frontend uses
/// this to register keyboard listeners and to populate the
/// shortcuts settings panel.
///
/// # Returns
///
/// A vector of [`ShortcutDefinition`] structs representing all
/// available shortcuts. The list is always populated (even if
/// no project is open), since defaults are defined statically.
///
/// # Customization
///
/// Shortcuts marked as `customizable: true` can be modified by the
/// user via [`set_shortcut`]. Non-customizable shortcuts retain
/// their default bindings permanently.
#[tauri::command]
pub fn get_shortcuts(
    project_state: State<ProjectState>,
) -> Result<Vec<ShortcutDefinition>, ShortcutError> {
    log::info!("Retrieving keyboard shortcuts");

    let project = project_state.data.lock().map_err(|e| ShortcutError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    let shortcuts = project.get_shortcuts().map_err(|e| ShortcutError {
        kind: "shortcuts_query_failed".into(),
        message: format!("Failed to retrieve shortcuts: {}", e),
    })?;

    let definitions = shortcuts
        .into_iter()
        .map(|s| ShortcutDefinition {
            action: s.action,
            key: s.key,
            modifiers: s.modifiers,
            description: s.description,
            context: s.context,
            customizable: s.customizable,
            default_key: s.default_key,
            default_modifiers: s.default_modifiers,
        })
        .collect();

    log::info!("Retrieved {} keyboard shortcuts", definitions.len());
    Ok(definitions)
}

/// Sets (customizes) a keyboard shortcut for a specific action.
///
/// Updates the key binding for the given action identifier. The
/// new binding is validated to ensure:
/// 1. The action exists and is marked as customizable.
/// 2. The key name is recognized.
/// 3. The modifier names are valid ("ctrl", "alt", "shift", "meta").
/// 4. The new binding does not conflict with any other shortcut.
///
/// # Parameters
///
/// - `action` — The action identifier to rebind (must match one
///   of the actions returned by [`get_shortcuts`]).
/// - `key` — The new primary key for this action.
/// - `modifiers` — The new modifier keys for this action. An empty
///   vector means the action triggers on the unmodified key alone.
///
/// # Returns
///
/// `true` if the shortcut was successfully updated, or a
/// [`ShortcutError`] if the action is not customizable, the
/// key/modifiers are invalid, or the binding conflicts with
/// another shortcut.
///
/// # Persistence
///
/// Custom shortcuts are stored as part of the project settings
/// and will be persisted when the project is saved. They are
/// also available globally as application-level preferences
/// for new projects.
///
/// # Conflict Resolution
///
/// If the new binding conflicts with another shortcut, the
/// command returns an error. The frontend should offer the
/// user the choice to:
/// - Swap the conflicting shortcuts.
/// - Clear the conflicting shortcut and apply the new one.
/// - Cancel the customization.
#[tauri::command]
pub fn set_shortcut(
    action: String,
    key: String,
    modifiers: Vec<String>,
    project_state: State<ProjectState>,
) -> Result<bool, ShortcutError> {
    log::info!(
        "Setting shortcut: action={}, key={}, modifiers={}",
        action,
        key,
        modifiers
    );

    if action.trim().is_empty() {
        return Err(ShortcutError {
            kind: "validation".into(),
            message: "Action identifier must not be empty.".into(),
        });
    }
    if key.trim().is_empty() {
        return Err(ShortcutError {
            kind: "validation".into(),
            message: "Key must not be empty.".into(),
        });
    }

    // Validate modifier names
    let valid_modifiers = ["ctrl", "alt", "shift", "meta"];
    for mod_key in &modifiers {
        if !valid_modifiers.contains(&mod_key.as_str()) {
            return Err(ShortcutError {
                kind: "validation".into(),
                message: format!(
                    "Invalid modifier '{}'. Valid modifiers are: ctrl, alt, shift, meta.",
                    mod_key
                ),
            });
        }
    }

    let mut project = project_state.data.lock().map_err(|e| ShortcutError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(ShortcutError {
            kind: "no_active_project".into(),
            message: "No active project. Shortcuts are managed per-project.".into(),
        });
    }

    // Check if the action is customizable
    let is_customizable = project
        .is_shortcut_customizable(&action)
        .map_err(|e| ShortcutError {
            kind: "action_not_found".into(),
            message: format!("Action '{}' not found: {}", action, e),
        })?;

    if !is_customizable {
        return Err(ShortcutError {
            kind: "not_customizable".into(),
            message: format!(
                "Action '{}' cannot be customized. It uses a locked default binding.",
                action
            ),
        });
    }

    // Check for conflicts with other shortcuts
    let conflict = project
        .find_shortcut_conflict(&key, &modifiers)
        .map_err(|e| ShortcutError {
            kind: "conflict_check_failed".into(),
            message: format!("Failed to check for shortcut conflicts: {}", e),
        })?;

    if let Some(conflicting_action) = conflict {
        return Err(ShortcutError {
            kind: "conflict".into(),
            message: format!(
                "Key '{}' with modifiers [{}] conflicts with action '{}'. Resolve the conflict before applying.",
                key,
                modifiers.join("+"),
                conflicting_action
            ),
        });
    }

    // Apply the new shortcut binding
    project
        .set_shortcut(&action, &key, &modifiers)
        .map_err(|e| ShortcutError {
            kind: "set_shortcut_failed".into(),
            message: format!("Failed to set shortcut: {}", e),
        })?;

    log::info!(
        "Shortcut for '{}' updated: {} + [{}]",
        action,
        key,
        modifiers.join("+")
    );
    Ok(true)
}
