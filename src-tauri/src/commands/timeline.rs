//! Timeline editing commands for FlowCut.
//!
//! This module provides Tauri command handlers for manipulating the
//! multi-track timeline, including adding/removing clips, splitting
//! and trimming clips, managing tracks, and creating transitions
//! between clips. All editing operations are undoable through the
//! [`UndoManager`] system.
//!
//! # State Dependencies
//!
//! Commands depend on [`ProjectState`] for the timeline data and
//! [`UndoManager`] for reversible operations.

use serde::{Deserialize, Serialize};
use tauri::State;

use flowcut_lib::project::ProjectState;
use flowcut_lib::utils::UndoManager;

/// Describes a clip placed on the timeline.
///
/// A clip is a reference to a segment of a media item, positioned
/// at a specific start time on a track. Trim values define the
/// portion of the source media that is visible on the timeline.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClipInfo {
    /// Unique identifier for this clip (UUID v4).
    pub id: String,
    /// ID of the track this clip belongs to.
    pub track_id: String,
    /// ID of the source media item this clip references.
    pub media_id: String,
    /// Position on the timeline where this clip starts, in seconds.
    pub start_time: f64,
    /// Visible duration of this clip on the timeline, in seconds.
    pub duration: f64,
    /// In-point offset within the source media, in seconds.
    /// This is the amount trimmed from the beginning of the source.
    pub in_point: f64,
    /// Out-point offset within the source media, in seconds.
    /// `in_point + duration` equals the effective out-point.
    pub out_point: f64,
    /// Optional name label for this clip, overriding the media filename.
    pub label: Option<String>,
    /// Whether the clip is locked (cannot be moved or edited).
    pub locked: bool,
    /// Whether the clip is muted (audio silenced / video hidden).
    pub muted: bool,
    /// Volume level for audio clips, from 0.0 (silent) to 2.0 (boosted).
    pub volume: f64,
    /// Opacity for video clips, from 0.0 (transparent) to 1.0 (opaque).
    pub opacity: f64,
}

/// Describes a track on the timeline.
///
/// Tracks are horizontal lanes that hold clips. Video tracks are
/// composited in order (bottom to top), and audio tracks are mixed.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackInfo {
    /// Unique identifier for this track (UUID v4).
    pub id: String,
    /// Track type: "video" or "audio".
    pub track_type: String,
    /// Human-readable name for this track (e.g. "Video 1", "Audio Master").
    pub name: String,
    /// Whether this track is locked (clips cannot be modified).
    pub locked: bool,
    /// Whether this track is visible/audible in the preview.
    pub visible: bool,
    /// Volume level for audio tracks (0.0–2.0).
    pub volume: f64,
    /// Opacity for video tracks (0.0–1.0).
    pub opacity: f64,
    /// Sort order index — lower values are rendered first.
    pub order: u32,
    /// Number of clips currently on this track.
    pub clip_count: usize,
}

/// Describes a transition between two clips.
///
/// Transitions blend the end of one clip into the beginning of the next,
/// such as cross-fades, wipes, or dissolves.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransitionInfo {
    /// Unique identifier for this transition (UUID v4).
    pub id: String,
    /// ID of the clip the transition starts from.
    pub from_clip_id: String,
    /// ID of the clip the transition leads into.
    pub to_clip_id: String,
    /// Type of transition effect (e.g. "crossfade", "dissolve", "wipe_left").
    pub transition_type: String,
    /// Duration of the transition overlap in seconds.
    pub duration: f64,
}

/// A snapshot of the complete timeline state.
///
/// Contains all tracks, their clips, and transitions, providing
/// the frontend with everything needed to render the timeline view.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelineState {
    /// All tracks on the timeline, ordered by sort index.
    pub tracks: Vec<TrackInfo>,
    /// All clips across all tracks.
    pub clips: Vec<ClipInfo>,
    /// All transitions between clips.
    pub transitions: Vec<TransitionInfo>,
    /// Total timeline duration in seconds (furthest clip end point).
    pub duration: f64,
    /// Current playback cursor position in seconds.
    pub cursor_position: f64,
}

/// Error type for timeline command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimelineError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Adds a clip to a track on the timeline.
///
/// Creates a new clip referencing the specified media item and places
/// it at the given start time on the target track. The clip's duration
/// defaults to the remaining duration of the source media (adjusted by
/// any existing trims), but can be overridden.
///
/// # Parameters
///
/// - `track_id` — ID of the track to place the clip on.
/// - `media_id` — ID of the source media item.
/// - `start_time` — Position on the timeline in seconds where the clip begins.
/// - `duration` — Duration of the clip in seconds. If this exceeds the
///   source media duration, it is clamped.
///
/// # Returns
///
/// A [`ClipInfo`] struct describing the newly created clip, or a
/// [`TimelineError`] if the track or media ID is invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action; undoing will remove the clip.
#[tauri::command]
pub fn add_clip_to_track(
    track_id: String,
    media_id: String,
    start_time: f64,
    duration: f64,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<ClipInfo, TimelineError> {
    log::info!(
        "Adding clip: track={}, media={}, start={}, duration={}",
        track_id, media_id, start_time, duration
    );

    if track_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track ID must not be empty.".into(),
        });
    }
    if media_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Media ID must not be empty.".into(),
        });
    }
    if start_time < 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Start time must not be negative.".into(),
        });
    }
    if duration <= 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Clip duration must be positive.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let result = project
        .add_clip(&track_id, &media_id, start_time, duration)
        .map_err(|e| TimelineError {
            kind: "add_clip_failed".into(),
            message: format!("Failed to add clip: {}", e),
        })?;

    undo_manager
        .record_action("add_clip_to_track", serde_json::json!({
            "clip_id": result.id.clone(),
            "track_id": track_id.clone(),
            "media_id": media_id.clone(),
            "start_time": start_time,
            "duration": duration,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(ClipInfo {
        id: result.id,
        track_id: result.track_id,
        media_id: result.media_id,
        start_time: result.start_time,
        duration: result.duration,
        in_point: result.in_point,
        out_point: result.out_point,
        label: result.label,
        locked: result.locked,
        muted: result.muted,
        volume: result.volume,
        opacity: result.opacity,
    })
}

/// Removes a clip from a track on the timeline.
///
/// The clip is deleted from the track and any transitions referencing
/// it are also removed. This is a destructive operation that can be
/// reversed via undo.
///
/// # Parameters
///
/// - `track_id` — ID of the track containing the clip.
/// - `clip_id` — ID of the clip to remove.
///
/// # Returns
///
/// `true` if the clip was successfully removed, or a [`TimelineError`]
/// if the clip or track does not exist.
///
/// # Undo Support
///
/// Recorded as an undoable action; undoing will restore the clip and
/// any transitions that were removed as a side effect.
#[tauri::command]
pub fn remove_clip_from_track(
    track_id: String,
    clip_id: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<bool, TimelineError> {
    log::info!("Removing clip: track={}, clip={}", track_id, clip_id);

    if track_id.trim().is_empty() || clip_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track ID and Clip ID must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Fetch clip info before removal for undo recording
    let clip_info = project.get_clip(&clip_id).map_err(|e| TimelineError {
        kind: "clip_not_found".into(),
        message: format!("Clip '{}' not found: {}", clip_id, e),
    })?;

    project
        .remove_clip(&track_id, &clip_id)
        .map_err(|e| TimelineError {
            kind: "remove_clip_failed".into(),
            message: format!("Failed to remove clip: {}", e),
        })?;

    undo_manager
        .record_action("remove_clip_from_track", serde_json::json!({
            "clip_id": clip_id.clone(),
            "track_id": track_id.clone(),
            "media_id": clip_info.media_id.clone(),
            "start_time": clip_info.start_time,
            "duration": clip_info.duration,
            "in_point": clip_info.in_point,
            "out_point": clip_info.out_point,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Clip '{}' removed successfully", clip_id);
    Ok(true)
}

/// Moves a clip to a new position on the timeline.
///
/// Changes the start time of the clip without altering its duration
/// or trim points. If the new position would cause an overlap with
/// another clip on the same track, the behavior depends on the
/// project's overlap resolution mode (ripple, insert, or overwrite).
///
/// # Parameters
///
/// - `track_id` — ID of the track containing the clip.
/// - `clip_id` — ID of the clip to move.
/// - `new_start_time` — The desired new start time in seconds.
///
/// # Returns
///
/// `true` if the clip was successfully moved, or a [`TimelineError`]
/// if the clip does not exist or `new_start_time` is invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action storing the original start time.
#[tauri::command]
pub fn move_clip(
    track_id: String,
    clip_id: String,
    new_start_time: f64,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<bool, TimelineError> {
    log::info!(
        "Moving clip: track={}, clip={}, new_start={}",
        track_id, clip_id, new_start_time
    );

    if track_id.trim().is_empty() || clip_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track ID and Clip ID must not be empty.".into(),
        });
    }
    if new_start_time < 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "New start time must not be negative.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Record the original position for undo
    let clip_info = project.get_clip(&clip_id).map_err(|e| TimelineError {
        kind: "clip_not_found".into(),
        message: format!("Clip '{}' not found: {}", clip_id, e),
    })?;
    let original_start_time = clip_info.start_time;

    project
        .move_clip(&track_id, &clip_id, new_start_time)
        .map_err(|e| TimelineError {
            kind: "move_clip_failed".into(),
            message: format!("Failed to move clip: {}", e),
        })?;

    undo_manager
        .record_action("move_clip", serde_json::json!({
            "clip_id": clip_id.clone(),
            "track_id": track_id.clone(),
            "original_start_time": original_start_time,
            "new_start_time": new_start_time,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Clip '{}' moved to start_time={}", clip_id, new_start_time);
    Ok(true)
}

/// Splits a clip at a given timestamp, producing two clips.
///
/// The original clip is removed and replaced by two new clips:
/// one covering the segment before the split point, and one
/// covering the segment after. Transitions attached to the
/// original clip are reassigned to the appropriate resulting clip.
///
/// # Parameters
///
/// - `track_id` — ID of the track containing the clip.
/// - `clip_id` — ID of the clip to split.
/// - `split_time` — The absolute timeline timestamp at which to
///   split. This must be within the clip's `[start_time, start_time + duration)`
///   range.
///
/// # Returns
///
/// A vector of two [`ClipInfo`] structs representing the left and
/// right halves of the split, or a [`TimelineError`] if the clip
/// does not exist or `split_time` is outside the clip's range.
///
/// # Undo Support
///
/// Recorded as an undoable action; undoing will merge the two clips
/// back into the original.
#[tauri::command]
pub fn split_clip(
    track_id: String,
    clip_id: String,
    split_time: f64,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<Vec<ClipInfo>, TimelineError> {
    log::info!(
        "Splitting clip: track={}, clip={}, split_time={}",
        track_id, clip_id, split_time
    );

    if track_id.trim().is_empty() || clip_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track ID and Clip ID must not be empty.".into(),
        });
    }
    if split_time < 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Split time must not be negative.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Record original clip info for undo
    let original = project.get_clip(&clip_id).map_err(|e| TimelineError {
        kind: "clip_not_found".into(),
        message: format!("Clip '{}' not found: {}", clip_id, e),
    })?;

    // Validate that split_time is within the clip's timeline range
    if split_time <= original.start_time || split_time >= original.start_time + original.duration {
        return Err(TimelineError {
            kind: "validation".into(),
            message: format!(
                "Split time {} must be within the clip range [{}, {}).",
                split_time,
                original.start_time,
                original.start_time + original.duration
            ),
        });
    }

    let results = project
        .split_clip(&track_id, &clip_id, split_time)
        .map_err(|e| TimelineError {
            kind: "split_clip_failed".into(),
            message: format!("Failed to split clip: {}", e),
        })?;

    let clip_infos: Vec<ClipInfo> = results
        .iter()
        .map(|r| ClipInfo {
            id: r.id,
            track_id: r.track_id,
            media_id: r.media_id,
            start_time: r.start_time,
            duration: r.duration,
            in_point: r.in_point,
            out_point: r.out_point,
            label: r.label,
            locked: r.locked,
            muted: r.muted,
            volume: r.volume,
            opacity: r.opacity,
        })
        .collect();

    undo_manager
        .record_action("split_clip", serde_json::json!({
            "original_clip_id": clip_id.clone(),
            "track_id": track_id.clone(),
            "split_time": split_time,
            "new_clip_ids": clip_infos.iter().map(|c| c.id.clone()).collect::<Vec<String>>(),
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Clip '{}' split into 2 clips", clip_id);
    Ok(clip_infos)
}

/// Trims a clip by adjusting its in-point and out-point.
///
/// Trimming changes which portion of the source media is visible
/// on the timeline without changing the clip's timeline position.
/// Positive `start_trim` removes content from the beginning,
/// positive `end_trim` removes content from the end.
///
/// # Parameters
///
/// - `track_id` — ID of the track containing the clip.
/// - `clip_id` — ID of the clip to trim.
/// - `start_trim` — Seconds to trim from the start of the clip.
///   Must be non-negative and less than the current clip duration.
/// - `end_trim` — Seconds to trim from the end of the clip.
///   Must be non-negative. The remaining duration after both trims
///   must be positive.
///
/// # Returns
///
/// Updated [`ClipInfo`] reflecting the new trim values, or a
/// [`TimelineError`] if the trim values are invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action storing original trim values.
#[tauri::command]
pub fn trim_clip(
    track_id: String,
    clip_id: String,
    start_trim: f64,
    end_trim: f64,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<ClipInfo, TimelineError> {
    log::info!(
        "Trimming clip: track={}, clip={}, start_trim={}, end_trim={}",
        track_id, clip_id, start_trim, end_trim
    );

    if track_id.trim().is_empty() || clip_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track ID and Clip ID must not be empty.".into(),
        });
    }
    if start_trim < 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Start trim must not be negative.".into(),
        });
    }
    if end_trim < 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "End trim must not be negative.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Record original clip info for undo
    let original = project.get_clip(&clip_id).map_err(|e| TimelineError {
        kind: "clip_not_found".into(),
        message: format!("Clip '{}' not found: {}", clip_id, e),
    })?;

    // Validate that the remaining duration after trimming is positive
    let remaining_duration = original.duration - start_trim - end_trim;
    if remaining_duration <= 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: format!(
                "Trim values would result in zero or negative duration (remaining: {:.3}s).",
                remaining_duration
            ),
        });
    }

    let result = project
        .trim_clip(&track_id, &clip_id, start_trim, end_trim)
        .map_err(|e| TimelineError {
            kind: "trim_clip_failed".into(),
            message: format!("Failed to trim clip: {}", e),
        })?;

    undo_manager
        .record_action("trim_clip", serde_json::json!({
            "clip_id": clip_id.clone(),
            "track_id": track_id.clone(),
            "original_duration": original.duration,
            "original_in_point": original.in_point,
            "original_out_point": original.out_point,
            "start_trim": start_trim,
            "end_trim": end_trim,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(ClipInfo {
        id: result.id,
        track_id: result.track_id,
        media_id: result.media_id,
        start_time: result.start_time,
        duration: result.duration,
        in_point: result.in_point,
        out_point: result.out_point,
        label: result.label,
        locked: result.locked,
        muted: result.muted,
        volume: result.volume,
        opacity: result.opacity,
    })
}

/// Retrieves the complete state of the timeline.
///
/// Returns all tracks, clips, and transitions along with the
/// overall timeline duration and cursor position. This is the
/// primary command the frontend calls to render the timeline
/// after any editing operation.
///
/// # Returns
///
/// A [`TimelineState`] snapshot, or a [`TimelineError`] if there
/// is no active project.
#[tauri::command]
pub fn get_timeline_state(
    project_state: State<ProjectState>,
) -> Result<TimelineState, TimelineError> {
    log::info!("Retrieving timeline state");

    let project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let state = project.get_timeline_state().map_err(|e| TimelineError {
        kind: "timeline_state_failed".into(),
        message: format!("Failed to get timeline state: {}", e),
    })?;

    Ok(TimelineState {
        tracks: state
            .tracks
            .into_iter()
            .map(|t| TrackInfo {
                id: t.id,
                track_type: t.track_type,
                name: t.name,
                locked: t.locked,
                visible: t.visible,
                volume: t.volume,
                opacity: t.opacity,
                order: t.order,
                clip_count: t.clip_count,
            })
            .collect(),
        clips: state
            .clips
            .into_iter()
            .map(|c| ClipInfo {
                id: c.id,
                track_id: c.track_id,
                media_id: c.media_id,
                start_time: c.start_time,
                duration: c.duration,
                in_point: c.in_point,
                out_point: c.out_point,
                label: c.label,
                locked: c.locked,
                muted: c.muted,
                volume: c.volume,
                opacity: c.opacity,
            })
            .collect(),
        transitions: state
            .transitions
            .into_iter()
            .map(|t| TransitionInfo {
                id: t.id,
                from_clip_id: t.from_clip_id,
                to_clip_id: t.to_clip_id,
                transition_type: t.transition_type,
                duration: t.duration,
            })
            .collect(),
        duration: state.duration,
        cursor_position: state.cursor_position,
    })
}

/// Adds a new track to the timeline.
///
/// Creates a new video or audio track with the specified name and
/// appends it to the timeline. The track's sort order is set
/// automatically based on the existing track count.
///
/// # Parameters
///
/// - `track_type` — Must be "video" or "audio". Determines the
///   track's lane type and compositing behavior.
/// - `name` — A human-readable name for the track (e.g. "Overlay",
///   "Background Music").
///
/// # Returns
///
/// A [`TrackInfo`] struct describing the newly created track, or a
/// [`TimelineError`] if `track_type` is invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action; undoing will remove the track.
#[tauri::command]
pub fn add_track(
    track_type: String,
    name: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<TrackInfo, TimelineError> {
    log::info!("Adding track: type={}, name={}", track_type, name);

    if track_type != "video" && track_type != "audio" {
        return Err(TimelineError {
            kind: "validation".into(),
            message: format!(
                "Track type must be 'video' or 'audio', got '{}'.",
                track_type
            ),
        });
    }
    if name.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track name must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let result = project
        .add_track(&track_type, &name)
        .map_err(|e| TimelineError {
            kind: "add_track_failed".into(),
            message: format!("Failed to add track: {}", e),
        })?;

    undo_manager
        .record_action("add_track", serde_json::json!({
            "track_id": result.id.clone(),
            "track_type": track_type.clone(),
            "name": name.clone(),
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(TrackInfo {
        id: result.id,
        track_type: result.track_type,
        name: result.name,
        locked: result.locked,
        visible: result.visible,
        volume: result.volume,
        opacity: result.opacity,
        order: result.order,
        clip_count: result.clip_count,
    })
}

/// Removes a track from the timeline.
///
/// Removes the track and all clips on it. Any transitions involving
/// clips on this track are also removed. This is a destructive
/// operation that can be reversed via undo.
///
/// # Parameters
///
/// - `track_id` — ID of the track to remove.
///
/// # Returns
///
/// `true` if the track was successfully removed, or a [`TimelineError`]
/// if the track does not exist.
///
/// # Undo Support
///
/// Recorded as an undoable action storing the full track state for
/// potential restoration.
#[tauri::command]
pub fn remove_track(
    track_id: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<bool, TimelineError> {
    log::info!("Removing track: {}", track_id);

    if track_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Track ID must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Fetch track info before removal for undo recording
    let track_info = project.get_track(&track_id).map_err(|e| TimelineError {
        kind: "track_not_found".into(),
        message: format!("Track '{}' not found: {}", track_id, e),
    })?;

    project
        .remove_track(&track_id)
        .map_err(|e| TimelineError {
            kind: "remove_track_failed".into(),
            message: format!("Failed to remove track: {}", e),
        })?;

    undo_manager
        .record_action("remove_track", serde_json::json!({
            "track_id": track_id.clone(),
            "track_type": track_info.track_type.clone(),
            "name": track_info.name.clone(),
            "order": track_info.order,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Track '{}' removed successfully", track_id);
    Ok(true)
}

/// Adds a transition between two adjacent clips.
///
/// Creates a transition effect that blends the end of `from_clip`
/// into the beginning of `to_clip`. The two clips must be on the
/// same track and adjacent (or overlapping) in timeline position.
///
/// # Parameters
///
/// - `from_clip` — ID of the outgoing clip.
/// - `to_clip` — ID of the incoming clip.
/// - `transition_type` — The type of transition to apply. Supported
///   values include: "crossfade", "dissolve", "wipe_left",
///   "wipe_right", "wipe_up", "wipe_down", "slide_left", "slide_right",
///   "zoom_in", "zoom_out".
/// - `duration` — Duration of the transition overlap in seconds.
///   Must be positive and less than either clip's remaining duration.
///
/// # Returns
///
/// A [`TransitionInfo`] struct describing the newly created
/// transition, or a [`TimelineError`] if the clips are not adjacent,
/// the transition type is unsupported, or the duration is invalid.
///
/// # Undo Support
///
/// Recorded as an undoable action; undoing will remove the transition.
#[tauri::command]
pub fn add_transition(
    from_clip: String,
    to_clip: String,
    transition_type: String,
    duration: f64,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<TransitionInfo, TimelineError> {
    log::info!(
        "Adding transition: from={}, to={}, type={}, duration={}",
        from_clip, to_clip, transition_type, duration
    );

    if from_clip.trim().is_empty() || to_clip.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Clip IDs must not be empty.".into(),
        });
    }
    if transition_type.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Transition type must not be empty.".into(),
        });
    }
    if duration <= 0.0 {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Transition duration must be positive.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    let result = project
        .add_transition(&from_clip, &to_clip, &transition_type, duration)
        .map_err(|e| TimelineError {
            kind: "add_transition_failed".into(),
            message: format!("Failed to add transition: {}", e),
        })?;

    undo_manager
        .record_action("add_transition", serde_json::json!({
            "transition_id": result.id.clone(),
            "from_clip": from_clip.clone(),
            "to_clip": to_clip.clone(),
            "transition_type": transition_type.clone(),
            "duration": duration,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    Ok(TransitionInfo {
        id: result.id,
        from_clip_id: result.from_clip_id,
        to_clip_id: result.to_clip_id,
        transition_type: result.transition_type,
        duration: result.duration,
    })
}

/// Removes a transition from the timeline.
///
/// Removes the transition and restores the original clip boundaries
/// that were adjusted to accommodate the transition overlap.
///
/// # Parameters
///
/// - `transition_id` — ID of the transition to remove.
///
/// # Returns
///
/// `true` if the transition was successfully removed, or a
/// [`TimelineError`] if the transition ID does not exist.
///
/// # Undo Support
///
/// Recorded as an undoable action storing transition details for
/// potential restoration.
#[tauri::command]
pub fn remove_transition(
    transition_id: String,
    project_state: State<ProjectState>,
    undo_manager: State<UndoManager>,
) -> Result<bool, TimelineError> {
    log::info!("Removing transition: {}", transition_id);

    if transition_id.trim().is_empty() {
        return Err(TimelineError {
            kind: "validation".into(),
            message: "Transition ID must not be empty.".into(),
        });
    }

    let mut project = project_state.data.lock().map_err(|e| TimelineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    if !project.is_open() {
        return Err(TimelineError {
            kind: "no_active_project".into(),
            message: "No active project.".into(),
        });
    }

    // Fetch transition info before removal for undo recording
    let transition_info = project
        .get_transition(&transition_id)
        .map_err(|e| TimelineError {
            kind: "transition_not_found".into(),
            message: format!("Transition '{}' not found: {}", transition_id, e),
        })?;

    project
        .remove_transition(&transition_id)
        .map_err(|e| TimelineError {
            kind: "remove_transition_failed".into(),
            message: format!("Failed to remove transition: {}", e),
        })?;

    undo_manager
        .record_action("remove_transition", serde_json::json!({
            "transition_id": transition_id.clone(),
            "from_clip": transition_info.from_clip_id.clone(),
            "to_clip": transition_info.to_clip_id.clone(),
            "transition_type": transition_info.transition_type.clone(),
            "duration": transition_info.duration,
        }))
        .map_err(|e| TimelineError {
            kind: "undo_record".into(),
            message: format!("Failed to record undo action: {}", e),
        })?;

    log::info!("Transition '{}' removed successfully", transition_id);
    Ok(true)
}
