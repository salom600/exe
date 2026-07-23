//! Preview rendering commands for FlowCut.
//!
//! This module provides Tauri command handlers for the real-time preview
//! system. The preview engine decodes and renders timeline frames at a
//! given timestamp, returning them as base64-encoded images that the
//! frontend can display in the preview viewport. It also supports seeking
//! operations and retrieving preview metadata for a given media item.
//!
//! # State Dependencies
//!
//! Commands depend on [`EngineState`] for FFmpeg decoding operations
//! and [`ProjectState`] for timeline and media references.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::engine::EngineState;
use crate::project::ProjectState;

/// Metadata about the preview capabilities for a media item.
///
/// Contains information needed by the frontend to configure the
/// preview viewport, such as the native resolution, aspect ratio,
/// and supported playback frame rate.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreviewInfo {
    /// ID of the media item this preview info refers to.
    pub media_id: String,
    /// Native video width in pixels.
    pub width: u32,
    /// Native video height in pixels.
    pub height: u32,
    /// Aspect ratio as a float (e.g. 1.778 for 16:9, 1.333 for 4:3).
    pub aspect_ratio: f64,
    /// Native frame rate in frames per second.
    pub frame_rate: f64,
    /// Total duration in seconds.
    pub duration: f64,
    /// Total number of frames.
    pub total_frames: u64,
    /// Supported preview resolutions (e.g. ["1920x1080", "1280x720", "640x360"]).
    pub available_resolutions: Vec<String>,
    /// Whether this media item supports scrubbing (random-access decode).
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
/// timestamp (applying effects, transitions, and opacity blending)
/// and returns the resulting frame as a base64-encoded PNG image.
/// This is the primary command called by the frontend during playback
/// or scrubbing to update the preview viewport.
///
/// # Parameters
///
/// - `timestamp` — The timeline position in seconds at which to
///   render the preview frame. Must be within `[0, timeline_duration]`.
///
/// # Returns
///
/// A base64-encoded string containing the PNG image data of the
/// rendered frame. The frontend can decode this directly into an
/// `<img>` element's `src` attribute using the data URL format:
/// `data:image/png;base64,<returned_string>`.
///
/// # Performance Considerations
///
/// Rendering a full composite frame is computationally expensive.
/// The frontend should:
/// - Cache rendered frames for repeated timestamps.
/// - Use a reduced preview resolution during scrubbing.
/// - Batch frame requests during playback rather than requesting
///   each frame individually.
///
/// # Errors
///
/// Returns a [`PreviewError`] if the engine is not initialized,
/// there is no active project, the timestamp is out of range,
/// or a decode/render failure occurs.
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

    // Acquire engine state lock
    let mut engine = engine_state.data.lock().map_err(|e| PreviewError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    // Verify the engine is initialized
    if !engine.is_initialized() {
        return Err(PreviewError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized. Call initialize_engine first."
                .into(),
        });
    }

    // Acquire project state lock for timeline reference
    let project = project_state.data.lock().map_err(|e| PreviewError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(PreviewError {
            kind: "no_active_project".into(),
            message: "No active project. Open a project before rendering previews.".into(),
        });
    }

    // Validate timestamp is within timeline bounds
    let timeline_duration = project.get_timeline_duration().map_err(|e| PreviewError {
        kind: "duration_query_failed".into(),
        message: format!("Failed to query timeline duration: {}", e),
    })?;

    if timestamp > timeline_duration {
        return Err(PreviewError {
            kind: "validation".into(),
            message: format!(
                "Timestamp {:.3}s exceeds timeline duration {:.3}s.",
                timestamp, timeline_duration
            ),
        });
    }

    // Render the composite frame at the given timestamp
    let frame_data = engine
        .render_frame(timestamp, &project)
        .map_err(|e| PreviewError {
            kind: "render_failed".into(),
            message: format!("Failed to render preview frame: {}", e),
        })?;

    log::debug!(
        "Preview frame rendered: {:.3}s, {} bytes base64",
        timestamp,
        frame_data.len()
    );

    Ok(frame_data)
}

/// Retrieves preview metadata for a specific media item.
///
/// Returns information about the media's native resolution, frame rate,
/// and decoding capabilities. The frontend uses this to configure the
/// preview viewport dimensions and to determine whether scrubbing
/// (random-access frame seeking) is supported for this media type.
///
/// # Parameters
///
/// - `media_id` — The UUID of the media item to query.
///
/// # Returns
///
/// A [`PreviewInfo`] struct containing the preview metadata, or a
/// [`PreviewError`] if the media item does not exist or the engine
/// cannot decode it.
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

    let engine = engine_state.data.lock().map_err(|e| PreviewError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    if !engine.is_initialized() {
        return Err(PreviewError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized.".into(),
        });
    }

    let project = project_state.data.lock().map_err(|e| PreviewError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(PreviewError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Retrieve media metadata from the project
    let media = project.get_media(&media_id).map_err(|e| PreviewError {
        kind: "media_not_found".into(),
        message: format!("Media item '{}' not found: {}", media_id, e),
    })?;

    // Query the engine for decoding capabilities
    let engine_info = engine
        .get_preview_capabilities(&media_id)
        .map_err(|e| PreviewError {
            kind: "preview_info_failed".into(),
            message: format!("Failed to get preview info: {}", e),
        })?;

    let width = media.width.unwrap_or(1920);
    let height = media.height.unwrap_or(1080);
    let aspect_ratio = if height > 0 {
        width as f64 / height as f64
    } else {
        16.0 / 9.0
    };

    Ok(PreviewInfo {
        media_id: media_id.clone(),
        width,
        height,
        aspect_ratio,
        frame_rate: media.frame_rate.unwrap_or(30.0),
        duration: media.duration,
        total_frames: media.total_frames.unwrap_or(0),
        available_resolutions: engine_info.available_resolutions,
        supports_scrubbing: engine_info.supports_scrubbing,
        codec: media.video_codec.unwrap_or_else(|| "unknown".to_string()),
    })
}

/// Seeks the preview playback cursor to a specified timestamp.
///
/// Updates the engine's internal playback position so that subsequent
/// calls to [`render_preview_frame`] will decode from the new position
/// efficiently (leveraging FFmpeg's seek mechanism rather than linear
/// decode from the beginning). This is essential for smooth scrubbing
/// performance.
///
/// # Parameters
///
/// - `timestamp` — The timeline position in seconds to seek to.
///   Must be within `[0, timeline_duration]`.
///
/// # Returns
///
/// `true` if the seek was successful, or a [`PreviewError`] if the
/// engine is not initialized, there is no active project, or the
/// timestamp is out of bounds.
///
/// # Usage Pattern
///
/// The frontend should call `seek_preview` before rendering frames
/// during scrubbing operations:
/// ```text
/// seek_preview(timestamp) → render_preview_frame(timestamp)
/// ```
/// This ensures the FFmpeg decoder is positioned at the correct
/// timestamp, avoiding expensive linear decode from the start.
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

    let mut engine = engine_state.data.lock().map_err(|e| PreviewError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    if !engine.is_initialized() {
        return Err(PreviewError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized.".into(),
        });
    }

    let project = project_state.data.lock().map_err(|e| PreviewError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(PreviewError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Validate timestamp bounds
    let timeline_duration = project.get_timeline_duration().map_err(|e| PreviewError {
        kind: "duration_query_failed".into(),
        message: format!("Failed to query timeline duration: {}", e),
    })?;

    if timestamp > timeline_duration {
        return Err(PreviewError {
            kind: "validation".into(),
            message: format!(
                "Seek timestamp {:.3}s exceeds timeline duration {:.3}s.",
                timestamp, timeline_duration
            ),
        });
    }

    engine.seek(timestamp).map_err(|e| PreviewError {
        kind: "seek_failed".into(),
        message: format!("Failed to seek preview: {}", e),
    })?;

    log::info!("Preview seeked to {:.3}s successfully", timestamp);
    Ok(true)
}
