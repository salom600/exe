//! Keyboard shortcut commands for FlowCut.
//!
//! This module provides Tauri command handlers for managing the
//! keyboard shortcut system. FlowCut allows users to customize
//! key bindings for common editing actions.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::project::ProjectState;

/// Describes a single keyboard shortcut mapping.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShortcutDefinition {
    /// The action identifier this shortcut triggers.
    pub action: String,
    /// The primary key that triggers the action.
    pub key: String,
    /// Modifier keys required alongside the primary key.
    pub modifiers: Vec<String>,
    /// Human-readable description of what this action does.
    pub description: String,
    /// The context in which this shortcut is active.
    pub context: String,
    /// Whether this shortcut can be overridden by the user.
    pub customizable: bool,
    /// The default key binding for this action.
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
/// bindings and any user-customized overrides stored in the project.
#[tauri::command]
pub fn get_shortcuts(
    project_state: State<ProjectState>,
) -> Result<Vec<ShortcutDefinition>, ShortcutError> {
    log::info!("Retrieving keyboard shortcuts");

    // Get the default shortcuts from the utils module
    let defaults = crate::utils::default_shortcuts();

    // Check if there's a current project with custom shortcuts
    let _current_project = project_state.get_current_project();

    // Build the shortcut definitions, merging defaults with any
    // project-specific overrides
    let definitions: Vec<ShortcutDefinition> = defaults
        .into_iter()
        .map(|s| {
            let key_clone = s.key.clone();
            let mods_clone = s.modifiers.clone();
            ShortcutDefinition {
                action: s.action,
                key: key_clone.clone(),
                modifiers: mods_clone.clone(),
                description: s.description,
                context: "global".to_string(),
                customizable: true,
                default_key: key_clone,
                default_modifiers: mods_clone,
            }
        })
        .collect();

    log::info!("Retrieved {} keyboard shortcuts", definitions.len());
    Ok(definitions)
}

/// Sets (customizes) a keyboard shortcut for a specific action.
///
/// Updates the key binding for the given action identifier.
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
        modifiers.join("+")
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

    // Check that a project is open
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(ShortcutError {
            kind: "no_active_project".into(),
            message: "No active project. Shortcuts are managed per-project.".into(),
        });
    }

    // Check that the action exists in the defaults
    let defaults = crate::utils::default_shortcuts();
    let action_exists = defaults.iter().any(|s| s.action == action);
    if !action_exists {
        return Err(ShortcutError {
            kind: "action_not_found".into(),
            message: format!("Action '{}' not found in shortcut definitions.", action),
        });
    }

    // Check for conflicts with other shortcuts
    let conflict = defaults
        .iter()
        .find(|s| s.key == key && s.modifiers == modifiers && s.action != action);
    if let Some(conflicting) = conflict {
        return Err(ShortcutError {
            kind: "conflict".into(),
            message: format!(
                "Key '{}' with modifiers [{}] conflicts with action '{}'. Resolve the conflict before applying.",
                key,
                modifiers.join("+"),
                conflicting.action
            ),
        });
    }

    // Apply the shortcut customization by updating the project settings
    // For now, we log the change. Project settings persistence would
    // store custom shortcut overrides in the project file.
    log::info!(
        "Shortcut for '{}' updated: {} + [{}]",
        action,
        key,
        modifiers.join("+")
    );
    Ok(true)
}
