//! Utility modules for FlowCut.
//!
//! This module provides shared utility structures used across the application,
//! including the undo/redo management system and keyboard shortcut definitions.
//!
//! # Sub-modules
//!
//! - **UndoManager**: Tracks user actions for undo/redo operations, maintaining
//!   separate stacks for undone and redone actions with a configurable history limit.
//! - **ShortcutDefinition**: Defines keyboard shortcut bindings for editor actions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Action tracking
// ---------------------------------------------------------------------------

/// A record of a user action that can be undone.
///
/// `ActionRecord` captures all information needed to reverse (undo) or
/// replay (redo) a user action on the timeline or project. Each record
/// stores the action's type, a human-readable description, the timestamp
/// when it occurred, and a JSON payload containing the data needed to
/// perform the undo/redo operation.
///
/// # Data Schema
///
/// The `data` field uses `serde_json::Value` to accommodate the diverse
/// data shapes required by different action types. The expected schema
/// varies by `action_type`:
///
/// | action_type          | data schema                                        |
/// |----------------------|----------------------------------------------------|
/// | "add_clip"           | `{"clip": {...}, "track_id": "..."}`               |
/// | "remove_clip"        | `{"clip": {...}, "track_id": "...", "index": N}`   |
/// | "move_clip"          | `{"clip_id": "...", "old_start": X, "new_start": Y}`|
/// | "split_clip"         | `{"original_clip": {...}, "new_clips": [...]}`     |
/// | "trim_clip"          | `{"clip_id": "...", "old_in": X, "old_out": Y, ...}`|
/// | "add_filter"         | `{"filter": {...}, "clip_id": "..."}`              |
/// | "remove_filter"      | `{"filter": {...}, "clip_id": "..."}`              |
/// | "add_track"          | `{"track": {...}}`                                 |
/// | "remove_track"       | `{"track": {...}}`                                 |
/// | "change_setting"     | `{"key": "...", "old_value": ..., "new_value": ...}`|
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// A unique identifier for this action record.
    pub id: Uuid,

    /// The type of action that was performed (e.g., "add_clip", "remove_clip",
    /// "move_clip", "split_clip", "trim_clip", "add_filter", "remove_filter",
    /// "add_track", "remove_track", "change_setting").
    pub action_type: String,

    /// A human-readable description of the action, suitable for displaying
    /// in the undo history UI (e.g., "Added clip 'intro.mp4' to Video 1",
    /// "Removed brightness filter from clip on Audio 2").
    pub description: String,

    /// The timestamp when this action was originally performed.
    ///
    /// This timestamp is preserved even after undo/redo cycles, so the
    /// history view always shows when the action first occurred.
    pub timestamp: DateTime<Utc>,

    /// The action-specific data payload containing all information needed
    /// to reverse or replay this action.
    ///
    /// See the `ActionRecord` documentation for the expected schema per action type.
    pub data: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Undo/Redo manager
// ---------------------------------------------------------------------------

/// The undo/redo management system for FlowCut editor actions.
///
/// `UndoManager` maintains two stacks — an undo stack and a redo stack —
/// that track user actions on the timeline and project. When the user
/// performs an action, it is pushed onto the undo stack and the redo stack
/// is cleared. When the user undoes an action, the top of the undo stack
/// is popped and pushed onto the redo stack. When the user redoes an action,
/// the top of the redo stack is popped and pushed back onto the undo stack.
///
/// # History Limit
///
/// The undo stack is capped at `max_history` entries (default: 100). When
/// the stack exceeds this limit, the oldest entries are discarded. This
/// prevents unbounded memory growth during long editing sessions.
///
/// # Thread Safety
///
/// All mutable fields are wrapped in `std::sync::Mutex` for safe concurrent
/// access from multiple Tauri command handlers. The undo manager is registered
/// as a Tauri managed state via `app.manage(UndoManager::new())`.
///
/// # Examples
///
/// ```rust
/// use flowcut_lib::utils::UndoManager;
/// use flowcut_lib::utils::ActionRecord;
/// use serde_json::json;
///
/// let manager = UndoManager::new();
///
/// // Record an action
/// let record = ActionRecord {
///     id: uuid::Uuid::new_v4(),
///     action_type: "add_clip".to_string(),
///     description: "Added clip to Video 1".to_string(),
///     timestamp: chrono::Utc::now(),
///     data: json!({"clip_id": "abc123"}),
/// };
/// manager.push_action(record);
///
/// // Undo it
/// let undone = manager.undo();
/// assert!(undone.is_some());
///
/// // Redo it
/// let redone = manager.redo();
/// assert!(redone.is_some());
/// ```
pub struct UndoManager {
    /// The stack of actions that can be undone.
    ///
    /// The most recent action is at the top (end) of the vector. When the
    /// user calls undo, the last entry is popped and moved to `redo_stack`.
    pub undo_stack: Mutex<Vec<ActionRecord>>,

    /// The stack of actions that can be redone.
    ///
    /// The most recently undone action is at the top (end) of the vector.
    /// When the user calls redo, the last entry is popped and moved back
    /// to `undo_stack`. This stack is cleared whenever a new action is
    /// recorded (pushed onto `undo_stack`).
    pub redo_stack: Mutex<Vec<ActionRecord>>,

    /// The maximum number of actions retained in the undo stack.
    ///
    /// When `undo_stack.len()` exceeds this value, the oldest entries are
    /// trimmed from the bottom of the stack. Default: 100.
    pub max_history: Mutex<usize>,
}

impl UndoManager {
    /// Creates a new `UndoManager` with empty stacks and a default history
    /// limit of 100 actions.
    ///
    /// # Returns
    ///
    /// A fresh `UndoManager` ready for Tauri state management.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flowcut_lib::utils::UndoManager;
    ///
    /// let manager = UndoManager::new();
    /// assert!(manager.undo_stack.lock().unwrap().is_empty());
    /// assert!(manager.redo_stack.lock().unwrap().is_empty());
    /// assert_eq!(*manager.max_history.lock().unwrap(), 100);
    /// ```
    pub fn new() -> Self {
        Self {
            undo_stack: Mutex::new(Vec::new()),
            redo_stack: Mutex::new(Vec::new()),
            max_history: Mutex::new(100),
        }
    }

    /// Records a new user action by pushing it onto the undo stack.
    ///
    /// When a new action is recorded, the redo stack is cleared because
    /// redo actions are no longer valid after a new action is performed
    /// (the redo branch diverges from the current state).
    ///
    /// If the undo stack exceeds `max_history`, the oldest entries are
    /// trimmed to maintain the limit.
    ///
    /// # Arguments
    ///
    /// * `record` - The `ActionRecord` describing the action to record.
    pub fn push_action(&self, record: ActionRecord) {
        // Clear the redo stack — new action invalidates redo history
        self.redo_stack.lock().unwrap().clear();

        // Push onto the undo stack
        let mut undo = self.undo_stack.lock().unwrap();
        undo.push(record);

        // Trim if exceeding max history
        let max = *self.max_history.lock().unwrap();
        if undo.len() > max {
            let excess = undo.len() - max;
            undo.drain(0..excess);
        }
    }

    /// Undoes the most recent action by popping it from the undo stack and
    /// pushing it onto the redo stack.
    ///
    /// # Returns
    ///
    /// `Some(ActionRecord)` containing the undone action's record, which
    /// the caller can use to reverse the action's effects on the project
    /// state. Returns `None` if the undo stack is empty (nothing to undo).
    pub fn undo(&self) -> Option<ActionRecord> {
        let mut undo = self.undo_stack.lock().unwrap();
        if let Some(record) = undo.pop() {
            self.redo_stack.lock().unwrap().push(record.clone());
            Some(record)
        } else {
            None
        }
    }

    /// Redoes the most recently undone action by popping it from the redo
    /// stack and pushing it back onto the undo stack.
    ///
    /// # Returns
    ///
    /// `Some(ActionRecord)` containing the redone action's record, which
    /// the caller can use to re-apply the action's effects on the project
    /// state. Returns `None` if the redo stack is empty (nothing to redo).
    pub fn redo(&self) -> Option<ActionRecord> {
        let mut redo = self.redo_stack.lock().unwrap();
        if let Some(record) = redo.pop() {
            self.undo_stack.lock().unwrap().push(record.clone());
            Some(record)
        } else {
            None
        }
    }

    /// Checks whether there are actions available to undo.
    ///
    /// # Returns
    ///
    /// `true` if the undo stack is non-empty, `false` otherwise.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.lock().unwrap().is_empty()
    }

    /// Checks whether there are actions available to redo.
    ///
    /// # Returns
    ///
    /// `true` if the redo stack is non-empty, `false` otherwise.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.lock().unwrap().is_empty()
    }

    /// Returns the number of actions currently in the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.lock().unwrap().len()
    }

    /// Returns the number of actions currently in the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.lock().unwrap().len()
    }

    /// Returns a cloned snapshot of the full undo history for display in the UI.
    ///
    /// The returned list is ordered from oldest to most recent action.
    pub fn get_undo_history(&self) -> Vec<ActionRecord> {
        self.undo_stack.lock().unwrap().clone()
    }

    /// Returns a cloned snapshot of the redo history for display in the UI.
    ///
    /// The returned list is ordered from oldest to most recently undone action.
    pub fn get_redo_history(&self) -> Vec<ActionRecord> {
        self.redo_stack.lock().unwrap().clone()
    }

    /// Clears both undo and redo stacks, resetting the undo history.
    ///
    /// Typically called when a new project is opened or when the user
    /// explicitly clears the undo history.
    pub fn clear(&self) {
        self.undo_stack.lock().unwrap().clear();
        self.redo_stack.lock().unwrap().clear();
    }

    /// Updates the maximum history limit.
    ///
    /// If the new limit is smaller than the current undo stack size,
    /// the oldest entries will be trimmed on the next `push_action` call.
    ///
    /// # Arguments
    ///
    /// * `max` - The new maximum number of undo entries to retain.
    pub fn set_max_history(&self, max: usize) {
        *self.max_history.lock().unwrap() = max;
    }
}

impl Default for UndoManager {
    /// Provides a default `UndoManager` equivalent to `UndoManager::new()`.
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts
// ---------------------------------------------------------------------------

/// A definition of a keyboard shortcut binding for an editor action.
///
/// `ShortcutDefinition` maps a keyboard key combination (key + modifiers)
/// to an editor action. Shortcuts are used to provide efficient keyboard-driven
/// access to common editing operations.
///
/// # Modifier Keys
///
/// The `modifiers` field uses string identifiers for platform-agnostic
/// modifier key representation:
///
/// - `"ctrl"` — Control key (Windows/Linux) or Command key (macOS)
/// - `"alt"` — Alt key (Windows/Linux) or Option key (macOS)
/// - `"shift"` — Shift key (all platforms)
/// - `"super"` — Windows key (Windows), Command key (macOS), Super key (Linux)
///
/// # Examples
///
/// ```rust
/// use flowcut_lib::utils::ShortcutDefinition;
///
/// let undo_shortcut = ShortcutDefinition {
///     action: "undo".to_string(),
///     key: "z".to_string(),
///     modifiers: vec!["ctrl".to_string()],
///     description: "Undo the last action".to_string(),
/// };
///
/// let split_shortcut = ShortcutDefinition {
///     action: "split_clip".to_string(),
///     key: "s".to_string(),
///     modifiers: vec!["ctrl", "shift"].into_iter().map(String::from).collect(),
///     description: "Split the clip at the cursor position".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutDefinition {
    /// The editor action identifier this shortcut triggers (e.g., "undo",
    /// "redo", "split_clip", "delete_clip", "play_pause", "zoom_in").
    pub action: String,

    /// The primary key for the shortcut (e.g., "z", "s", "Space", "Delete",
    /// "ArrowLeft"). Uses platform-standard key names.
    pub key: String,

    /// The modifier keys that must be held simultaneously with the primary key.
    /// See the `ShortcutDefinition` documentation for modifier key identifiers.
    pub modifiers: Vec<String>,

    /// A human-readable description of what this shortcut does, displayed in
    /// the shortcuts preference panel and tooltip overlays.
    pub description: String,
}

/// Returns the default set of keyboard shortcuts for FlowCut.
///
/// This function provides a comprehensive default shortcut mapping that covers
/// all major editing operations. The frontend can use this as the initial
/// shortcut configuration and allow the user to customize bindings via the
/// `set_shortcut` command.
///
/// # Returns
///
/// A `Vec<ShortcutDefinition>` containing all default keyboard shortcuts.
pub fn default_shortcuts() -> Vec<ShortcutDefinition> {
    vec![
        // Undo/Redo
        ShortcutDefinition {
            action: "undo".to_string(),
            key: "z".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Undo the last action".to_string(),
        },
        ShortcutDefinition {
            action: "redo".to_string(),
            key: "y".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Redo the last undone action".to_string(),
        },
        // Playback
        ShortcutDefinition {
            action: "play_pause".to_string(),
            key: "Space".to_string(),
            modifiers: vec![],
            description: "Toggle play/pause on the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "stop".to_string(),
            key: "Escape".to_string(),
            modifiers: vec![],
            description: "Stop playback and return to start".to_string(),
        },
        ShortcutDefinition {
            action: "seek_start".to_string(),
            key: "Home".to_string(),
            modifiers: vec![],
            description: "Seek to the beginning of the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "seek_end".to_string(),
            key: "End".to_string(),
            modifiers: vec![],
            description: "Seek to the end of the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "step_forward".to_string(),
            key: "ArrowRight".to_string(),
            modifiers: vec![],
            description: "Step forward one frame".to_string(),
        },
        ShortcutDefinition {
            action: "step_backward".to_string(),
            key: "ArrowLeft".to_string(),
            modifiers: vec![],
            description: "Step backward one frame".to_string(),
        },
        // Clip editing
        ShortcutDefinition {
            action: "split_clip".to_string(),
            key: "s".to_string(),
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            description: "Split the clip at the cursor position".to_string(),
        },
        ShortcutDefinition {
            action: "delete_clip".to_string(),
            key: "Delete".to_string(),
            modifiers: vec![],
            description: "Delete the selected clip".to_string(),
        },
        ShortcutDefinition {
            action: "ripple_delete".to_string(),
            key: "Delete".to_string(),
            modifiers: vec!["shift".to_string()],
            description: "Delete clip and close the gap (ripple delete)".to_string(),
        },
        ShortcutDefinition {
            action: "duplicate_clip".to_string(),
            key: "d".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Duplicate the selected clip".to_string(),
        },
        ShortcutDefinition {
            action: "copy".to_string(),
            key: "c".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Copy the selected clip(s) to clipboard".to_string(),
        },
        ShortcutDefinition {
            action: "paste".to_string(),
            key: "v".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Paste clipboard content at cursor position".to_string(),
        },
        ShortcutDefinition {
            action: "cut".to_string(),
            key: "x".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Cut the selected clip(s) to clipboard".to_string(),
        },
        // Track management
        ShortcutDefinition {
            action: "add_track".to_string(),
            key: "t".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Add a new track to the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "lock_track".to_string(),
            key: "l".to_string(),
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            description: "Lock/unlock the selected track".to_string(),
        },
        ShortcutDefinition {
            action: "mute_track".to_string(),
            key: "m".to_string(),
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            description: "Mute/unmute the selected audio track".to_string(),
        },
        // Zoom
        ShortcutDefinition {
            action: "zoom_in".to_string(),
            key: "+".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Zoom in on the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "zoom_out".to_string(),
            key: "-".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Zoom out on the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "zoom_fit".to_string(),
            key: "0".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Zoom to fit the entire timeline".to_string(),
        },
        // Markers
        ShortcutDefinition {
            action: "add_marker".to_string(),
            key: "m".to_string(),
            modifiers: vec![],
            description: "Add a marker at the current cursor position".to_string(),
        },
        // Export
        ShortcutDefinition {
            action: "export".to_string(),
            key: "e".to_string(),
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            description: "Open the export dialog".to_string(),
        },
        // Save
        ShortcutDefinition {
            action: "save_project".to_string(),
            key: "s".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Save the current project".to_string(),
        },
        ShortcutDefinition {
            action: "save_project_as".to_string(),
            key: "s".to_string(),
            modifiers: vec!["ctrl".to_string(), "shift".to_string()],
            description: "Save the project to a new file".to_string(),
        },
        // Selection
        ShortcutDefinition {
            action: "select_all".to_string(),
            key: "a".to_string(),
            modifiers: vec!["ctrl".to_string()],
            description: "Select all clips on the timeline".to_string(),
        },
        ShortcutDefinition {
            action: "select_none".to_string(),
            key: "Escape".to_string(),
            modifiers: vec!["shift".to_string()],
            description: "Clear the current selection".to_string(),
        },
    ]
}
