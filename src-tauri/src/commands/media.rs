//! Media management commands for FlowCut.
//!
//! This module provides Tauri command handlers for importing, querying,
//! and removing media files (video, audio, images) from the project's
//! media library. Imported media is analyzed by the engine to extract
//! metadata such as duration, resolution, codec, and frame rate, which
//! is stored alongside the media reference for efficient timeline editing.
//!
//! # State Dependencies
//!
//! Commands depend on [`ProjectState`] for the media library and
//! [`UndoManager`] for reversible operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use flowcut_lib::project::ProjectState;
use flowcut_lib::utils::UndoManager;

/// A complete descriptor for a media item in the project library.
///
/// Contains both the filesystem reference and all metadata extracted
/// during import. The frontend uses this struct to populate the media
/// browser panel and to determine clip properties when placing media
/// on the timeline.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MediaItem {
    /// Unique identifier for this media item (UUID v4).
    pub id: String,
    /// Original filename of the imported media (e.g. "clip_001.mp4").
    pub name: String,
    /// Absolute filesystem path to the source media file.
    pub path: String,
    /// Media type classification: "video", "audio", or "image".
    pub media_type: String,
    /// Duration in seconds. Zero for images and audio-only files
    /// that lack embedded duration metadata.
    pub duration: f64,
    /// Video width in pixels, if the media contains a video stream.
    pub width: Option<u32>,
    /// Video height in pixels, if the media contains a video stream.
    pub height: Option<u32>,
    /// Video codec name (e.g. "h264", "hevc", "vp9") if applicable.
    pub video_codec: Option<String>,
    /// Audio codec name (e.g. "aac", "opus", "mp3") if applicable.
    pub audio_codec: Option<String>,
    /// Frame rate in frames per second, for video media.
    pub frame_rate: Option<f64>,
    /// Total number of video frames, for video media.
    pub total_frames: Option<u64>,
    /// Bit depth for image media (e.g. 8, 16).
    pub bit_depth: Option<u32>,
    /// Sample rate in Hz for audio streams (e.g. 48000).
    pub sample_rate: Option<u32>,
    /// Number of audio channels (1 = mono, 2 = stereo, 6 = 5.1).
    pub audio_channels: Option<u32>,
    /// File size in bytes.
    pub file_size: u64,
    /// ISO 8601 timestamp when this media was imported.
    pub imported_at: String,
    /// Thumbnail path relative to the project directory, if a
    /// thumbnail was generated during import.
    pub thumbnail_path: Option<String>,
}

/// Error type for media command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MediaError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Imports one or more media files into the current project's library.
///
/// Each path is resolved to its filesystem location, analyzed by the
/// FFmpeg engine to extract metadata (codec, duration, resolution, etc.),
/// and registered in the project's media library. Thumbnails are generated
/// for video and image media for use in the media browser panel.
///
/// # Parameters
///
/// - `paths` — A list of absolute filesystem paths to media files.
///   Supported formats include MP4, MOV, MKV, AVI, WebM, MP3, WAV,
///   FLAC, OGG, PNG, JPEG, BMP, TIFF, and others supported by FFmpeg.
///
/// # Returns
///
/// A vector of [`MediaItem`] structs, one per successfully imported file.
/// Files that fail to import are skipped; the frontend should check that
/// the returned count matches the input count and alert the user to
/// any discrepancies.
///
/// # Undo Support
///
/// Each import is recorded as an undoable action. Calling [`undo_action`]
/// will remove the imported media item from the project library.
#[tauri::command]
pub fn import_media(
    paths: Vec<String>,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<Vec<MediaItem>, MediaError> {
    log::info!("Importing {} media file(s)", paths.len());

    if paths.is_empty() {
        return Err(MediaError {
            kind: "validation".into(),
            message: "No media paths provided. Pass at least one file path.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| MediaError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project. Open or create a project before importing media.".into(),
        });
    }

    let mut imported_items = Vec::with_capacity(paths.len());

    for file_path in &paths {
        if file_path.trim().is_empty() {
            log::warn!("Skipping empty path in import list");
            continue;
        }

        log::debug!("Importing media from: {}", file_path);

        let result = project.import_media(file_path).map_err(|e| {
            log::error!("Failed to import '{}': {}", file_path, e);
            MediaError {
                kind: "import_failed".into(),
                message: format!("Failed to import '{}': {}", file_path, e),
            }
        })?;

        // Record each import as an undoable action
        undo_manager
            .record_action("import_media", serde_json::json!({
                "media_id": result.id.clone(),
                "path": file_path.clone(),
            }))
            .map_err(|e| MediaError {
                kind: "undo_record".into(),
                message: format!("Failed to record undo action: {}", e),
            })?;

        imported_items.push(MediaItem {
            id: result.id,
            name: result.name,
            path: result.path,
            media_type: result.media_type,
            duration: result.duration,
            width: result.width,
            height: result.height,
            video_codec: result.video_codec,
            audio_codec: result.audio_codec,
            frame_rate: result.frame_rate,
            total_frames: result.total_frames,
            bit_depth: result.bit_depth,
            sample_rate: result.sample_rate,
            audio_channels: result.audio_channels,
            file_size: result.file_size,
            imported_at: result.imported_at,
            thumbnail_path: result.thumbnail_path,
        });
    }

    log::info!("Successfully imported {} media item(s)", imported_items.len());
    Ok(imported_items)
}

/// Retrieves detailed metadata for a specific media item.
///
/// Returns the full [`MediaItem`] descriptor for the media item identified
/// by the given ID. This is useful for the frontend to display detailed
/// media properties in a inspector panel or to validate that a media
/// reference is still valid before placing it on the timeline.
///
/// # Parameters
///
/// - `id` — The UUID of the media item to retrieve.
///
/// # Returns
///
/// The [`MediaItem`] struct for the requested media, or a [`MediaError`]
/// if the ID does not exist in the current project's library.
#[tauri::command]
pub fn get_media_info(
    id: String,
    project_state: State<ProjectState>,
) -> Result<MediaItem, MediaError> {
    log::info!("Retrieving media info for id: {}", id);

    if id.trim().is_empty() {
        return Err(MediaError {
            kind: "validation".into(),
            message: "Media ID must not be empty.".into(),
        });
    }

    let project = project_state.data.lock().map_err(|e| MediaError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let result = project.get_media(&id).map_err(|e| MediaError {
        kind: "media_not_found".into(),
        message: format!("Media item '{}' not found: {}", id, e),
    })?;

    Ok(MediaItem {
        id: result.id,
        name: result.name,
        path: result.path,
        media_type: result.media_type,
        duration: result.duration,
        width: result.width,
        height: result.height,
        video_codec: result.video_codec,
        audio_codec: result.audio_codec,
        frame_rate: result.frame_rate,
        total_frames: result.total_frames,
        bit_depth: result.bit_depth,
        sample_rate: result.sample_rate,
        audio_channels: result.audio_channels,
        file_size: result.file_size,
        imported_at: result.imported_at,
        thumbnail_path: result.thumbnail_path,
    })
}

/// Removes a media item from the project's library.
///
/// The media item is unregistered from the project and any associated
/// thumbnail cache is cleaned up. Note: this does **not** remove clips
/// on the timeline that reference this media — the frontend should
/// handle that separately or warn the user about orphaned clips.
///
/// # Parameters
///
/// - `id` — The UUID of the media item to remove.
///
/// # Returns
///
/// `true` if the media was successfully removed, or a [`MediaError`] if
/// the media ID does not exist or the project has no active workspace.
///
/// # Undo Support
///
/// Removing media is recorded as an undoable action, storing enough
/// information to re-import the media if the user performs an undo.
#[tauri::command]
pub fn remove_media(
    id: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<bool, MediaError> {
    log::info!("Removing media item with id: {}", id);

    if id.trim().is_empty() {
        return Err(MediaError {
            kind: "validation".into(),
            message: "Media ID must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| MediaError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Fetch media info before removal so we can record it for undo
    let media_info = project.get_media(&id).map_err(|e| MediaError {
        kind: "media_not_found".into(),
        message: format!("Media item '{}' not found: {}", id, e),
    })?;

    project.remove_media(&id).map_err(|e| MediaError {
        kind: "remove_failed".into(),
        message: format!("Failed to remove media item '{}': {}", id, e),
    })?;

    // Record the removal as an undoable action
    undo_manager
        .record_action("remove_media", serde_json::json!({
            "media_id": id.clone(),
            "path": media_info.path.clone(),
            "name": media_info.name.clone(),
        }))
        .map_err(|e| MediaError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Media item '{}' removed successfully", id);
    Ok(true)
}

/// Lists all media items in the current project's library.
///
/// Returns every [`MediaItem`] in the project, suitable for populating
/// the media browser panel on the frontend. The items are returned in
/// the order they were imported.
///
/// # Returns
///
/// A vector of [`MediaItem`] structs, or a [`MediaError`] if there is
/// no active project.
#[tauri::command]
pub fn list_media(
    project_state: State<ProjectState>,
) -> Result<Vec<MediaItem>, MediaError> {
    log::info!("Listing all media items");

    let project = project_state.data.lock().map_err(|e| MediaError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let media_list = project.list_media().map_err(|e| MediaError {
        kind: "list_failed".into(),
        message: format!("Failed to list media items: {}", e),
    })?;

    let items = media_list
        .into_iter()
        .map(|m| MediaItem {
            id: m.id,
            name: m.name,
            path: m.path,
            media_type: m.media_type,
            duration: m.duration,
            width: m.width,
            height: m.height,
            video_codec: m.video_codec,
            audio_codec: m.audio_codec,
            frame_rate: m.frame_rate,
            total_frames: m.total_frames,
            bit_depth: m.bit_depth,
            sample_rate: m.sample_rate,
            audio_channels: m.audio_channels,
            file_size: m.file_size,
            imported_at: m.imported_at,
            thumbnail_path: m.thumbnail_path,
        })
        .collect();

    log::info!("Listed {} media items", items.len());
    Ok(items)
}
