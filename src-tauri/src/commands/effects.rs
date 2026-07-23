//! Effects and filters commands for FlowCut.
//!
//! This module provides Tauri command handlers for applying, modifying,
//! and removing visual and audio effects (filters) on timeline clips.
//! Effects are non-destructive — they are applied as overlays on clip
//! references and can be reordered, reconfigured, or removed at any
//! time without altering the underlying media.
//!
//! # Filter Architecture
//!
//! Each filter instance is stored as a separate entity linked to a clip.
//! Multiple filters can be applied to a single clip, and they are
//! processed in order (top-to-bottom in the filter stack). Filter
//! parameters are stored as [`serde_json::Value`] to allow flexible,
//! extensible configuration without rigid type definitions.
//!
//! # State Dependencies
//!
//! Commands depend on [`ProjectState`] for filter storage and
//! [`UndoManager`] for reversible operations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use flowcut_lib::project::ProjectState;
use flowcut_lib::utils::UndoManager;

/// Describes a filter instance applied to a clip.
///
/// Contains the filter's unique ID, the clip it is attached to,
/// the filter type (e.g. "brightness", "blur", "equalizer"), and
/// the current parameter configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FilterInfo {
    /// Unique identifier for this filter instance (UUID v4).
    pub id: String,
    /// ID of the clip this filter is applied to.
    pub clip_id: String,
    /// The filter type identifier (e.g. "brightness", "contrast",
    /// "saturation", "blur", "sharpen", "noise_reduction",
    /// "equalizer", "compressor", "reverb").
    pub filter_type: String,
    /// Current parameter configuration as a JSON object.
    /// The schema varies per filter type; see [`FilterDefinition`]
    /// for the expected parameter structure.
    pub params: Value,
    /// Whether this filter is currently enabled. Disabled filters
    /// are skipped during rendering but retain their configuration.
    pub enabled: bool,
    /// Order index in the filter stack (0 = first applied).
    pub order: u32,
    /// Human-readable name for this filter instance, derived from
    /// the filter type and any user-provided label.
    pub name: String,
}

/// Describes a filter type definition (blueprint).
///
/// This is a static descriptor for a category of filters, containing
/// the type name, description, supported parameter schema, and
/// default parameter values. It does not represent an applied instance
/// — it is a template that [`apply_filter`] uses to create a [`FilterInfo`].
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FilterDefinition {
    /// The filter type identifier used in [`apply_filter`].
    pub filter_type: String,
    /// Human-readable category name (e.g. "Color", "Audio", "Stylize").
    pub category: String,
    /// Short description of what the filter does.
    pub description: String,
    /// JSON Schema describing the expected parameters for this
    /// filter type. The frontend can use this to generate a
    /// dynamic parameter editor UI.
    pub param_schema: Value,
    /// Default parameter values as a JSON object.
    pub default_params: Value,
    /// Whether this filter operates on video data.
    pub is_video_filter: bool,
    /// Whether this filter operates on audio data.
    pub is_audio_filter: bool,
}

/// Error type for effects/filters command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FilterError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Applies a filter to a clip on the timeline.
///
/// Creates a new filter instance of the specified type, attaches it to
/// the given clip, and configures it with the provided parameters. If
/// `params` is `null` or an empty object, the filter's default parameters
/// (from [`FilterDefinition::default_params`]) are used.
///
/// # Parameters
///
/// - `clip_id` — ID of the clip to apply the filter to.
/// - `filter_type` — The type of filter to apply (must match one of
///   the types returned by [`list_filters`]).
/// - `params` — A JSON object containing the filter's configuration.
///   Keys and value types must conform to the filter's parameter schema.
///   Pass `null` or `{}` to use defaults.
///
/// # Returns
///
/// A [`FilterInfo`] struct describing the newly created filter instance,
/// or a [`FilterError`] if the clip does not exist, the filter type is
/// unknown, or the parameters are invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action; undoing will remove the filter.
#[tauri::command]
pub fn apply_filter(
    clip_id: String,
    filter_type: String,
    params: Value,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<FilterInfo, FilterError> {
    log::info!(
        "Applying filter: clip={}, type={}, params={}",
        clip_id,
        filter_type,
        params
    );

    if clip_id.trim().is_empty() {
        return Err(FilterError {
            kind: "validation".into(),
            message: "Clip ID must not be empty.".into(),
        });
    }
    if filter_type.trim().is_empty() {
        return Err(FilterError {
            kind: "validation".into(),
            message: "Filter type must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| FilterError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Verify the clip exists
    let clip = project.get_clip(&clip_id).map_err(|e| FilterError {
        kind: "clip_not_found".into(),
        message: format!("Clip '{}' not found: {}", clip_id, e),
    })?;

    // Verify the filter type is supported
    project
        .validate_filter_type(&filter_type)
        .map_err(|e| FilterError {
            kind: "invalid_filter_type".into(),
            message: format!("Filter type '{}' is not supported: {}", filter_type, e),
        })?;

    // Apply default params if none were provided
    let effective_params =
        if params.is_null() || (params.is_object() && params.as_object().unwrap().is_empty()) {
            project
                .get_filter_defaults(&filter_type)
                .map_err(|e| FilterError {
                    kind: "defaults_failed".into(),
                    message: format!("Failed to get default params for '{}': {}", filter_type, e),
                })?
        } else {
            // Validate provided params against the filter's schema
            project
                .validate_filter_params(&filter_type, &params)
                .map_err(|e| FilterError {
                    kind: "invalid_params".into(),
                    message: format!("Invalid parameters for filter '{}': {}", filter_type, e),
                })?;
            params
        };

    let result = project
        .apply_filter(&clip_id, &filter_type, &effective_params)
        .map_err(|e| FilterError {
            kind: "apply_filter_failed".into(),
            message: format!("Failed to apply filter: {}", e),
        })?;

    undo_manager
        .record_action(
            "apply_filter",
            serde_json::json!({
                "filter_id": result.id.clone(),
                "clip_id": clip_id.clone(),
                "filter_type": filter_type.clone(),
                "params": effective_params.clone(),
            }),
        )
        .map_err(|e| FilterError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(FilterInfo {
        id: result.id,
        clip_id: result.clip_id,
        filter_type: result.filter_type,
        params: result.params,
        enabled: result.enabled,
        order: result.order,
        name: result.name,
    })
}

/// Removes a filter instance from a clip.
///
/// Deletes the filter instance identified by the given ID. The clip
/// returns to its previous rendering behavior without this filter's
/// effect. Any filters above this one in the stack shift down to
/// fill the gap in ordering.
///
/// # Parameters
///
/// - `filter_id` — The UUID of the filter instance to remove.
///
/// # Returns
///
/// `true` if the filter was successfully removed, or a [`FilterError`]
/// if the filter ID does not exist.
///
/// # Undo Support
///
/// Recorded as an undoable action storing the filter's full configuration
/// for potential restoration.
#[tauri::command]
pub fn remove_filter(
    filter_id: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<bool, FilterError> {
    log::info!("Removing filter: {}", filter_id);

    if filter_id.trim().is_empty() {
        return Err(FilterError {
            kind: "validation".into(),
            message: "Filter ID must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| FilterError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Fetch filter info before removal for undo recording
    let filter_info = project.get_filter(&filter_id).map_err(|e| FilterError {
        kind: "filter_not_found".into(),
        message: format!("Filter '{}' not found: {}", filter_id, e),
    })?;

    project.remove_filter(&filter_id).map_err(|e| FilterError {
        kind: "remove_filter_failed".into(),
        message: format!("Failed to remove filter: {}", e),
    })?;

    undo_manager
        .record_action(
            "remove_filter",
            serde_json::json!({
                "filter_id": filter_id.clone(),
                "clip_id": filter_info.clip_id.clone(),
                "filter_type": filter_info.filter_type.clone(),
                "params": filter_info.params.clone(),
                "order": filter_info.order,
            }),
        )
        .map_err(|e| FilterError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Filter '{}' removed successfully", filter_id);
    Ok(true)
}

/// Lists all available filter type definitions.
///
/// Returns the catalog of all filter types that FlowCut supports,
/// including their parameter schemas and default values. The frontend
/// uses this to populate the effects browser panel and to generate
/// dynamic parameter editor UIs.
///
/// # Returns
///
/// A vector of [`FilterDefinition`] structs, one per supported filter
/// type. The list is static and does not depend on project state.
#[tauri::command]
pub fn list_filters(
    project_state: State<ProjectState>,
) -> Result<Vec<FilterDefinition>, FilterError> {
    log::info!("Listing available filter definitions");

    let project = project_state.data.lock().map_err(|e| FilterError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    let definitions = project.list_filter_definitions().map_err(|e| FilterError {
        kind: "list_filters_failed".into(),
        message: format!("Failed to list filter definitions: {}", e),
    })?;

    let filter_defs = definitions
        .into_iter()
        .map(|d| FilterDefinition {
            filter_type: d.filter_type,
            category: d.category,
            description: d.description,
            param_schema: d.param_schema,
            default_params: d.default_params,
            is_video_filter: d.is_video_filter,
            is_audio_filter: d.is_audio_filter,
        })
        .collect();

    log::info!("Listed {} filter definitions", filter_defs.len());
    Ok(filter_defs)
}

/// Retrieves the current parameters of a filter instance.
///
/// Returns the parameter configuration as a JSON object. This is
/// useful for the frontend to populate a filter's parameter editor
/// with the current values when the user selects a filter in the
/// inspector panel.
///
/// # Parameters
///
/// - `filter_id` — The UUID of the filter instance to query.
///
/// # Returns
///
/// A [`serde_json::Value`] containing the filter's current parameters,
/// or a [`FilterError`] if the filter ID does not exist.
#[tauri::command]
pub fn get_filter_params(
    filter_id: String,
    project_state: State<ProjectState>,
) -> Result<Value, FilterError> {
    log::info!("Getting filter params for: {}", filter_id);

    if filter_id.trim().is_empty() {
        return Err(FilterError {
            kind: "validation".into(),
            message: "Filter ID must not be empty.".into(),
        });
    }

    let project = project_state.data.lock().map_err(|e| FilterError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let filter = project.get_filter(&filter_id).map_err(|e| FilterError {
        kind: "filter_not_found".into(),
        message: format!("Filter '{}' not found: {}", filter_id, e),
    })?;

    Ok(filter.params)
}

/// Updates the parameters of a filter instance.
///
/// Replaces the filter's parameter configuration with the provided
/// JSON object. The new parameters must conform to the filter type's
/// schema (as defined in [`FilterDefinition::param_schema`]).
///
/// # Parameters
///
/// - `filter_id` — The UUID of the filter instance to update.
/// - `params` — A JSON object containing the new parameter values.
///   Partial updates are supported: only the keys present in `params`
///   will be changed; missing keys retain their previous values.
///
/// # Returns
///
/// An updated [`FilterInfo`] struct reflecting the new parameters,
/// or a [`FilterError`] if the filter does not exist or the
/// parameters are invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action storing both the original and new
/// parameter values for precise restoration on undo.
#[tauri::command]
pub fn update_filter_params(
    filter_id: String,
    params: Value,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<FilterInfo, FilterError> {
    log::info!(
        "Updating filter params: filter={}, params={}",
        filter_id,
        params
    );

    if filter_id.trim().is_empty() {
        return Err(FilterError {
            kind: "validation".into(),
            message: "Filter ID must not be empty.".into(),
        });
    }

    if params.is_null() {
        return Err(FilterError {
            kind: "validation".into(),
            message: "Parameters must not be null.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| FilterError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Fetch the current filter info for undo recording
    let original_filter = project.get_filter(&filter_id).map_err(|e| FilterError {
        kind: "filter_not_found".into(),
        message: format!("Filter '{}' not found: {}", filter_id, e),
    })?;

    // Validate the new params against the filter type's schema
    project
        .validate_filter_params(&original_filter.filter_type, &params)
        .map_err(|e| FilterError {
            kind: "invalid_params".into(),
            message: format!(
                "Invalid parameters for filter '{}': {}",
                original_filter.filter_type, e
            ),
        })?;

    let result = project
        .update_filter_params(&filter_id, &params)
        .map_err(|e| FilterError {
            kind: "update_failed".into(),
            message: format!("Failed to update filter params: {}", e),
        })?;

    undo_manager
        .record_action(
            "update_filter_params",
            serde_json::json!({
                "filter_id": filter_id.clone(),
                "original_params": original_filter.params.clone(),
                "new_params": params.clone(),
            }),
        )
        .map_err(|e| FilterError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(FilterInfo {
        id: result.id,
        clip_id: result.clip_id,
        filter_type: result.filter_type,
        params: result.params,
        enabled: result.enabled,
        order: result.order,
        name: result.name,
    })
}
