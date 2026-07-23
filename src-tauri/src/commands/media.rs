//! Media management commands for FlowCut.
//!
//! This module provides Tauri command handlers for importing, querying,
//! and removing media files from the project's media library.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::project::{MediaItem as InternalMediaItem, MediaType, Project, ProjectState};
use crate::utils::{ActionRecord, UndoManager};

/// A complete descriptor for a media item in the project library.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MediaItem {
    /// Unique identifier for this media item (UUID v4).
    pub id: String,
    /// Original filename of the imported media.
    pub name: String,
    /// Absolute filesystem path to the source media file.
    pub path: String,
    /// Media type classification: "video", "audio", or "image".
    pub media_type: String,
    /// Duration in seconds.
    pub duration: f64,
    /// Video width in pixels.
    pub width: Option<u32>,
    /// Video height in pixels.
    pub height: Option<u32>,
    /// Video codec name.
    pub video_codec: Option<String>,
    /// Audio codec name.
    pub audio_codec: Option<String>,
    /// Frame rate in frames per second.
    pub frame_rate: Option<f64>,
    /// Total number of video frames.
    pub total_frames: Option<u64>,
    /// Bit depth for image media.
    pub bit_depth: Option<u32>,
    /// Sample rate in Hz for audio streams.
    pub sample_rate: Option<u32>,
    /// Number of audio channels.
    pub audio_channels: Option<u32>,
    /// File size in bytes.
    pub file_size: u64,
    /// ISO 8601 timestamp when this media was imported.
    pub imported_at: String,
    /// Thumbnail path relative to the project directory.
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

/// Helper to map an internal MediaItem to the command-level MediaItem.
fn internal_to_media_item(m: &InternalMediaItem) -> MediaItem {
    let media_type_str = match m.media_type {
        MediaType::Video => "video",
        MediaType::Audio => "audio",
        MediaType::Image => "image",
    };

    // The internal MediaItem has codec (single field), so we
    // map it to video_codec for video/image types
    let (video_codec, audio_codec) = match m.media_type {
        MediaType::Video => (Some(m.codec.clone()), None),
        MediaType::Audio => (None, Some(m.codec.clone())),
        MediaType::Image => (Some(m.codec.clone()), None),
    };

    MediaItem {
        id: m.id.to_string(),
        name: m.name.clone(),
        path: m.path.clone(),
        media_type: media_type_str.to_string(),
        duration: m.duration,
        width: if m.width > 0 { Some(m.width) } else { None },
        height: if m.height > 0 { Some(m.height) } else { None },
        video_codec,
        audio_codec,
        frame_rate: if m.frame_rate > 0.0 { Some(m.frame_rate) } else { None },
        total_frames: if m.frame_rate > 0.0 && m.duration > 0.0 {
            Some((m.duration * m.frame_rate) as u64)
        } else {
            None
        },
        bit_depth: None,
        sample_rate: None,
        audio_channels: None,
        file_size: m.file_size,
        imported_at: chrono::Utc::now().to_rfc3339(), // Using current time as imported_at
        thumbnail_path: m.thumbnail_path.clone(),
    }
}

/// Imports one or more media files into the current project's library.
///
/// Each path is resolved to its filesystem location and registered
/// in the project's media library.
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

    // Get current project
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project. Open or create a project before importing media.".into(),
        });
    }

    let mut project = current_project.unwrap();
    let mut imported_items = Vec::with_capacity(paths.len());

    for file_path in &paths {
        if file_path.trim().is_empty() {
            log::warn!("Skipping empty path in import list");
            continue;
        }

        log::debug!("Importing media from: {}", file_path);

        // Create a new MediaItem for the imported file
        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Determine media type from extension
        let extension = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let media_type = if matches!(
            extension.as_str(),
            "mp4" | "mov" | "mkv" | "avi" | "webm" | "flv" | "ts" | "m4v" | "wmv"
        ) {
            MediaType::Video
        } else if matches!(
            extension.as_str(),
            "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" | "wma"
        ) {
            MediaType::Audio
        } else {
            MediaType::Image
        };

        let media_item = InternalMediaItem {
            id: uuid::Uuid::new_v4(),
            name: filename,
            path: file_path.clone(),
            media_type,
            duration: 0.0, // Would be analyzed by FFmpeg in a real implementation
            width: 0,
            height: 0,
            frame_rate: 0.0,
            codec: String::new(),
            bitrate: 0,
            file_size: 0,
            thumbnail_path: None,
        };

        // Record the import as an undoable action
        undo_manager.push_action(ActionRecord {
            id: uuid::Uuid::new_v4(),
            action_type: "import_media".to_string(),
            description: format!("Imported media '{}'", filename),
            timestamp: chrono::Utc::now(),
            data: serde_json::json!({
                "media_id": media_item.id.to_string(),
                "path": file_path.clone(),
            }),
        });

        imported_items.push(internal_to_media_item(&media_item));
        project.media_pool.push(media_item);
    }

    // Update the project in state
    project_state.update_project(project);

    log::info!(
        "Successfully imported {} media item(s)",
        imported_items.len()
    );
    Ok(imported_items)
}

/// Retrieves detailed metadata for a specific media item.
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

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let project = current_project.unwrap();

    // Parse the ID to UUID
    let parsed_uuid = uuid::Uuid::parse_str(&id).map_err(|e| MediaError {
        kind: "invalid_id".into(),
        message: format!("Invalid media ID format: {}", e),
    })?;

    // Find the media item in the project's media pool
    let media = project
        .media_pool
        .iter()
        .find(|m| m.id == parsed_uuid);

    if media.is_none() {
        return Err(MediaError {
            kind: "media_not_found".into(),
            message: format!("Media item '{}' not found.", id),
        });
    }

    Ok(internal_to_media_item(media.unwrap()))
}

/// Removes a media item from the project's library.
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

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let mut project = current_project.unwrap();

    // Parse the ID to UUID
    let parsed_uuid = uuid::Uuid::parse_str(&id).map_err(|e| MediaError {
        kind: "invalid_id".into(),
        message: format!("Invalid media ID format: {}", e),
    })?;

    // Find the media item before removal for undo recording
    let media_idx = project
        .media_pool
        .iter()
        .position(|m| m.id == parsed_uuid);

    if media_idx.is_none() {
        return Err(MediaError {
            kind: "media_not_found".into(),
            message: format!("Media item '{}' not found.", id),
        });
    }

    let removed_media = project.media_pool.remove(media_idx.unwrap());

    // Record the removal as an undoable action
    undo_manager.push_action(ActionRecord {
        id: uuid::Uuid::new_v4(),
        action_type: "remove_media".to_string(),
        description: format!("Removed media '{}'", removed_media.name),
        timestamp: chrono::Utc::now(),
        data: serde_json::json!({
            "media_id": id.clone(),
            "path": removed_media.path.clone(),
            "name": removed_media.name.clone(),
        }),
    });

    // Update the project in state
    project_state.update_project(project);

    log::info!("Media item '{}' removed successfully", id);
    Ok(true)
}

/// Lists all media items in the current project's library.
#[tauri::command]
pub fn list_media(project_state: State<ProjectState>) -> Result<Vec<MediaItem>, MediaError> {
    log::info!("Listing all media items");

    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(MediaError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let project = current_project.unwrap();

    let items: Vec<MediaItem> = project
        .media_pool
        .iter()
        .map(internal_to_media_item)
        .collect();

    log::info!("Listed {} media items", items.len());
    Ok(items)
}
