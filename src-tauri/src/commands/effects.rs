//! Effects and filters commands for FlowCut.
//!
//! This module provides Tauri command handlers for applying, modifying,
//! and removing visual and audio effects (filters) on timeline clips.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::project::{Clip, FilterInstance, ProjectState, Track};
use crate::utils::{ActionRecord, UndoManager};

/// Describes a filter instance applied to a clip.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FilterInfo {
    /// Unique identifier for this filter instance (UUID v4).
    pub id: String,
    /// ID of the clip this filter is applied to.
    pub clip_id: String,
    /// The filter type identifier.
    pub filter_type: String,
    /// Current parameter configuration as a JSON object.
    pub params: Value,
    /// Whether this filter is currently enabled.
    pub enabled: bool,
    /// Order index in the filter stack (0 = first applied).
    pub order: u32,
    /// Human-readable name for this filter instance.
    pub name: String,
}

/// Describes a filter type definition (blueprint).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FilterDefinition {
    /// The filter type identifier.
    pub filter_type: String,
    /// Human-readable category name.
    pub category: String,
    /// Short description of what the filter does.
    pub description: String,
    /// JSON Schema describing the expected parameters.
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

/// Static list of available filter definitions.
fn get_static_filter_definitions() -> Vec<FilterDefinition> {
    vec![
        FilterDefinition {
            filter_type: "brightness".to_string(),
            category: "Color".to_string(),
            description: "Adjust the brightness of the video".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "number", "minimum": -1.0, "maximum": 1.0 }
                }
            }),
            default_params: serde_json::json!({"amount": 0.0}),
            is_video_filter: true,
            is_audio_filter: false,
        },
        FilterDefinition {
            filter_type: "contrast".to_string(),
            category: "Color".to_string(),
            description: "Adjust the contrast of the video".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "number", "minimum": -1.0, "maximum": 2.0 }
                }
            }),
            default_params: serde_json::json!({"amount": 0.0}),
            is_video_filter: true,
            is_audio_filter: false,
        },
        FilterDefinition {
            filter_type: "saturation".to_string(),
            category: "Color".to_string(),
            description: "Adjust the saturation of the video".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "number", "minimum": -1.0, "maximum": 2.0 }
                }
            }),
            default_params: serde_json::json!({"amount": 0.0}),
            is_video_filter: true,
            is_audio_filter: false,
        },
        FilterDefinition {
            filter_type: "blur".to_string(),
            category: "Stylize".to_string(),
            description: "Apply a Gaussian blur to the video".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "radius": { "type": "number", "minimum": 0.0, "maximum": 100.0 },
                    "type": { "type": "string", "enum": ["gaussian", "box"] }
                }
            }),
            default_params: serde_json::json!({"radius": 5.0, "type": "gaussian"}),
            is_video_filter: true,
            is_audio_filter: false,
        },
        FilterDefinition {
            filter_type: "sharpen".to_string(),
            category: "Stylize".to_string(),
            description: "Sharpen the video edges".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "number", "minimum": 0.0, "maximum": 5.0 }
                }
            }),
            default_params: serde_json::json!({"amount": 1.0}),
            is_video_filter: true,
            is_audio_filter: false,
        },
        FilterDefinition {
            filter_type: "noise_reduction".to_string(),
            category: "Stylize".to_string(),
            description: "Reduce noise in the video".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "strength": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
                }
            }),
            default_params: serde_json::json!({"strength": 0.5}),
            is_video_filter: true,
            is_audio_filter: false,
        },
        FilterDefinition {
            filter_type: "equalizer".to_string(),
            category: "Audio".to_string(),
            description: "Apply an audio equalizer".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "bands": { "type": "array" }
                }
            }),
            default_params: serde_json::json!({"bands": []}),
            is_video_filter: false,
            is_audio_filter: true,
        },
        FilterDefinition {
            filter_type: "compressor".to_string(),
            category: "Audio".to_string(),
            description: "Apply dynamic range compression".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "threshold": { "type": "number" },
                    "ratio": { "type": "number" }
                }
            }),
            default_params: serde_json::json!({"threshold": -20.0, "ratio": 4.0}),
            is_video_filter: false,
            is_audio_filter: true,
        },
        FilterDefinition {
            filter_type: "reverb".to_string(),
            category: "Audio".to_string(),
            description: "Apply audio reverb effect".to_string(),
            param_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "decay": { "type": "number" },
                    "mix": { "type": "number" }
                }
            }),
            default_params: serde_json::json!({"decay": 0.5, "mix": 0.3}),
            is_video_filter: false,
            is_audio_filter: true,
        },
    ]
}

/// Applies a filter to a clip on the timeline.
///
/// Creates a new filter instance of the specified type, attaches it to
/// the given clip, and configures it with the provided parameters.
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

    // Get current project
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let mut project = current_project.unwrap();

    // Verify the filter type is supported
    let definitions = get_static_filter_definitions();
    let filter_def = definitions.iter().find(|d| d.filter_type == filter_type);
    if filter_def.is_none() {
        return Err(FilterError {
            kind: "invalid_filter_type".into(),
            message: format!("Filter type '{}' is not supported.", filter_type),
        });
    }

    let filter_def = filter_def.unwrap();

    // Apply default params if none were provided
    let effective_params =
        if params.is_null() || (params.is_object() && params.as_object().unwrap().is_empty()) {
            filter_def.default_params.clone()
        } else {
            params
        };

    // Parse the clip ID to UUID
    let parsed_clip_uuid = uuid::Uuid::parse_str(&clip_id).map_err(|e| FilterError {
        kind: "invalid_clip_id".into(),
        message: format!("Invalid clip ID format: {}", e),
    })?;

    // Find the clip and add the filter instance to it
    let filter_instance = FilterInstance {
        id: uuid::Uuid::new_v4(),
        filter_type: filter_type.clone(),
        params: effective_params.clone(),
        enabled: true,
        order: 0, // will be set to the next order in the clip's filter list
    };

    let mut clip_found = false;
    let mut filter_order = 0;

    for track in &mut project.timeline.tracks {
        for clip in &mut track.clips {
            if clip.id == parsed_clip_uuid {
                filter_order = clip.filters.len() as u32;
                let mut new_filter = filter_instance.clone();
                new_filter.order = filter_order;
                clip.filters.push(new_filter);
                clip_found = true;
                break;
            }
        }
        if clip_found {
            break;
        }
    }

    if !clip_found {
        return Err(FilterError {
            kind: "clip_not_found".into(),
            message: format!("Clip '{}' not found.", clip_id),
        });
    }

    // Record the action for undo support
    undo_manager.push_action(ActionRecord {
        id: uuid::Uuid::new_v4(),
        action_type: "apply_filter".to_string(),
        description: format!("Applied '{}' filter to clip", filter_type),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "filter_id": filter_instance.id.to_string(),
            "clip_id": clip_id.clone(),
            "filter_type": filter_type.clone(),
            "params": effective_params.clone(),
        }),
    });

    // Update the project
    project_state.update_project(project);

    Ok(FilterInfo {
        id: filter_instance.id.to_string(),
        clip_id: clip_id.clone(),
        filter_type: filter_type.clone(),
        params: effective_params,
        enabled: true,
        order: filter_order,
        name: format!("{} #{}", filter_type, filter_instance.id),
    })
}

/// Removes a filter instance from a clip.
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

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let mut project = current_project.unwrap();

    // Parse the filter ID to UUID
    let parsed_filter_uuid = uuid::Uuid::parse_str(&filter_id).map_err(|e| FilterError {
        kind: "invalid_filter_id".into(),
        message: format!("Invalid filter ID format: {}", e),
    })?;

    // Find and remove the filter
    let mut filter_found = false;
    let mut removed_filter: Option<FilterInstance> = None;
    let mut found_clip_id: Option<String> = None;

    for track in &mut project.timeline.tracks {
        for clip in &mut track.clips {
            let filter_idx = clip.filters.iter().position(|f| f.id == parsed_filter_uuid);
            if let Some(idx) = filter_idx {
                removed_filter = Some(clip.filters.remove(idx));
                found_clip_id = Some(clip.id.to_string());
                filter_found = true;
                // Re-order remaining filters
                for (i, f) in clip.filters.iter_mut().enumerate() {
                    f.order = i as u32;
                }
                break;
            }
        }
        if filter_found {
            break;
        }
    }

    if !filter_found {
        return Err(FilterError {
            kind: "filter_not_found".into(),
            message: format!("Filter '{}' not found.", filter_id),
        });
    }

    let removed = removed_filter.unwrap();
    let clip_id_str = found_clip_id.unwrap();

    // Record the removal as an undoable action
    undo_manager.push_action(ActionRecord {
        id: uuid::Uuid::new_v4(),
        action_type: "remove_filter".to_string(),
        description: format!("Removed '{}' filter", removed.filter_type),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "filter_id": filter_id.clone(),
            "clip_id": clip_id_str.clone(),
            "filter_type": removed.filter_type.clone(),
            "params": removed.params.clone(),
            "order": removed.order,
        }),
    });

    // Update the project
    project_state.update_project(project);

    log::info!("Filter '{}' removed successfully", filter_id);
    Ok(true)
}

/// Lists all available filter type definitions.
#[tauri::command]
pub fn list_filters(
    _project_state: State<ProjectState>,
) -> Result<Vec<FilterDefinition>, FilterError> {
    log::info!("Listing available filter definitions");

    // Return the static filter definitions
    let definitions = get_static_filter_definitions();

    log::info!("Listed {} filter definitions", definitions.len());
    Ok(definitions)
}

/// Retrieves the current parameters of a filter instance.
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

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let project = current_project.unwrap();

    // Parse the filter ID to UUID
    let parsed_filter_uuid = uuid::Uuid::parse_str(&filter_id).map_err(|e| FilterError {
        kind: "invalid_filter_id".into(),
        message: format!("Invalid filter ID format: {}", e),
    })?;

    // Find the filter in the project
    for track in &project.timeline.tracks {
        for clip in &track.clips {
            for filter in &clip.filters {
                if filter.id == parsed_filter_uuid {
                    return Ok(filter.params.clone());
                }
            }
        }
    }

    Err(FilterError {
        kind: "filter_not_found".into(),
        message: format!("Filter '{}' not found.", filter_id),
    })
}

/// Updates the parameters of a filter instance.
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

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(FilterError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let mut project = current_project.unwrap();

    // Parse the filter ID to UUID
    let parsed_filter_uuid = uuid::Uuid::parse_str(&filter_id).map_err(|e| FilterError {
        kind: "invalid_filter_id".into(),
        message: format!("Invalid filter ID format: {}", e),
    })?;

    // Find and update the filter
    let mut filter_found = false;
    let mut original_params: Option<Value> = None;
    let mut result_info: Option<FilterInfo> = None;
    let mut found_clip_id: Option<String> = None;

    for track in &mut project.timeline.tracks {
        for clip in &mut track.clips {
            for filter in &mut clip.filters {
                if filter.id == parsed_filter_uuid {
                    original_params = Some(filter.params.clone());
                    filter.params = params.clone();
                    result_info = Some(FilterInfo {
                        id: filter.id.to_string(),
                        clip_id: clip.id.to_string(),
                        filter_type: filter.filter_type.clone(),
                        params: filter.params.clone(),
                        enabled: filter.enabled,
                        order: filter.order,
                        name: format!("{} #{}", filter.filter_type, filter.id),
                    });
                    found_clip_id = Some(clip.id.to_string());
                    filter_found = true;
                    break;
                }
            }
            if filter_found {
                break;
            }
        }
        if filter_found {
            break;
        }
    }

    if !filter_found {
        return Err(FilterError {
            kind: "filter_not_found".into(),
            message: format!("Filter '{}' not found.", filter_id),
        });
    }

    // Record the update as an undoable action
    undo_manager.push_action(ActionRecord {
        id: uuid::Uuid::new_v4(),
        action_type: "update_filter_params".to_string(),
        description: format!("Updated filter '{}' params", filter_id),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "filter_id": filter_id.clone(),
            "original_params": original_params.unwrap(),
            "new_params": params.clone(),
        }),
    });

    // Update the project
    project_state.update_project(project);

    Ok(result_info.unwrap())
}
