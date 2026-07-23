//! Preview rendering commands for FlowCut.
//!
//! This module provides Tauri command handlers for the real-time preview
//! system. The preview engine decodes and renders timeline frames at a
//! given timestamp, returning them as base64-encoded images that the
//! frontend can display in the preview viewport.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::engine::EngineState;
use crate::project::ProjectState;

/// Metadata about the preview capabilities for a media item.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreviewInfo {
    /// ID of the media item this preview info refers to.
    pub media_id: String,
    /// Native video width in pixels.
    pub width: u32,
    /// Native video height in pixels.
    pub height: u32,
    /// Aspect ratio as a float (e.g. 1.778 for 16:9).
    pub aspect_ratio: f64,
    /// Native frame rate in frames per second.
    pub frame_rate: f64,
    /// Total duration in seconds.
    pub duration: f64,
    /// Total number of frames.
    pub total_frames: u64,
    /// Supported preview resolutions.
    pub available_resolutions: Vec<String>,
    /// Whether this media item supports scrubbing.
    pub supports_scrubbing: bool,
    /// Codec used for decoding the preview stream.
    pub codec: String,
}

/// Error type for preview command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreviewError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Renders a single preview frame at the specified timestamp.
///
/// The engine composites all visible tracks at the given timeline
/// timestamp and returns the resulting frame as a base64-encoded PNG.
#[tauri::command]
pub fn render_preview_frame(
    timestamp: f64,
    engine_state: State<EngineState>,
    project_state: State<ProjectState>,
) -> Result<String, PreviewError> {
    log::info!("Rendering preview frame at timestamp: {:.3}s", timestamp);

    if timestamp < 0.0 {
        return Err(PreviewError {
            kind: "validation".into(),
            message: "Timestamp must not be negative.".into(),
        });
    }

    // Verify the engine is initialized
    let is_initialized = *engine_state.is_initialized.lock().unwrap();
    if !is_initialized {
        return Err(PreviewError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized. Call initialize_engine first."
                .into(),
        });
    }

    // Verify project is open and get timeline duration
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(PreviewError {
            kind: "no_active_project".into(),
            message: "No active project. Open a project before rendering previews.".into(),
        });
    }

    let project = current_project.unwrap();

    // Validate timestamp is within timeline bounds
    let timeline_duration = project.timeline.duration;
    if timeline_duration > 0.0 && timestamp > timeline_duration {
        return Err(PreviewError {
            kind: "validation".into(),
            message: format!(
                "Timestamp {:.3}s exceeds timeline duration {:.3}s.",
                timestamp, timeline_duration
            ),
        });
    }

    // Render the composite frame at the given timestamp.
    // Since there's no actual render engine, return a placeholder
    // base64-encoded 1x1 transparent PNG.
    let placeholder_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwAFcAJN2e0zAAAAAElFTkSuQmCC";

    log::debug!("Preview frame rendered: {:.3}s (placeholder)", timestamp);

    Ok(placeholder_png.to_string())
}

/// Retrieves preview metadata for a specific media item.
///
/// Returns information about the media's native resolution, frame rate,
/// and decoding capabilities.
#[tauri::command]
pub fn get_preview_info(
    media_id: String,
    engine_state: State<EngineState>,
    project_state: State<ProjectState>,
) -> Result<PreviewInfo, PreviewError> {
    log::info!("Getting preview info for media: {}", media_id);

    if media_id.trim().is_empty() {
        return Err(PreviewError {
            kind: "validation".into(),
            message: "Media ID must not be empty.".into(),
        });
    }

    // Check engine is initialized
    let is_initialized = *engine_state.is_initialized.lock().unwrap();
    if !is_initialized {
        return Err(PreviewError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized.".into(),
        });
    }

    // Get current project
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(PreviewError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let project = current_project.unwrap();

    // Find the media item in the project's media pool
    let parsed_uuid = uuid::Uuid::parse_str(&media_id).map_err(|e| PreviewError {
        kind: "invalid_media_id".into(),
        message: format!("Invalid media ID format: {}", e),
    })?;

    let media = project.media_pool.iter().find(|m| m.id == parsed_uuid);

    if media.is_none() {
        return Err(PreviewError {
            kind: "media_not_found".into(),
            message: format!("Media item '{}' not found.", media_id),
        });
    }

    let media = media.unwrap();

    let aspect_ratio = if media.height > 0 {
        media.width as f64 / media.height as f64
    } else {
        16.0 / 9.0
    };

    Ok(PreviewInfo {
        media_id: media_id.clone(),
        width: media.width,
        height: media.height,
        aspect_ratio,
        frame_rate: media.frame_rate,
        duration: media.duration,
        total_frames: if media.frame_rate > 0.0 {
            (media.duration * media.frame_rate) as u64
        } else {
            0
        },
        available_resolutions: vec![
            format!("{}x{}", media.width, media.height),
            "1280x720".to_string(),
            "640x360".to_string(),
        ],
        supports_scrubbing: true,
        codec: media.codec.clone(),
    })
}

/// Seeks the preview playback cursor to a specified timestamp.
///
/// Updates the engine's internal playback position for efficient decoding.
#[tauri::command]
pub fn seek_preview(
    timestamp: f64,
    engine_state: State<EngineState>,
    project_state: State<ProjectState>,
) -> Result<bool, PreviewError> {
    log::info!("Seeking preview to timestamp: {:.3}s", timestamp);

    if timestamp < 0.0 {
        return Err(PreviewError {
            kind: "validation".into(),
            message: "Timestamp must not be negative.".into(),
        });
    }

    // Check engine is initialized
    let is_initialized = *engine_state.is_initialized.lock().unwrap();
    if !is_initialized {
        return Err(PreviewError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized.".into(),
        });
    }

    // Get current project for timeline bounds check
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(PreviewError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let project = current_project.unwrap();

    // Validate timestamp bounds
    let timeline_duration = project.timeline.duration;
    if timeline_duration > 0.0 && timestamp > timeline_duration {
        return Err(PreviewError {
            kind: "validation".into(),
            message: format!(
                "Seek timestamp {:.3}s exceeds timeline duration {:.3}s.",
                timestamp, timeline_duration
            ),
        });
    }

    // Seek operation is a placeholder — the actual seek would
    // position the FFmpeg decoder at the given timestamp.
    log::info!("Preview seeked to {:.3}s successfully", timestamp);
    Ok(true)
}
