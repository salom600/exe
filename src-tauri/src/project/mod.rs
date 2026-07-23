//! Project management state module for FlowCut.
//!
//! This module defines all data structures related to project management,
//! including the project itself, its timeline, tracks, clips, media items,
//! filters, markers, and settings. The `ProjectState` struct serves as the
//! Tauri-managed state container that holds the currently open project and
//! the list of recently accessed projects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The type of content a track carries on the timeline.
///
/// Each track in the timeline is classified as one of these types, which
/// determines how clips on that track are rendered and which operations
/// are valid for them.
///
/// - **Video** tracks contain visual content that is composited in the
///   video rendering pipeline.
/// - **Audio** tracks contain sound content that is mixed in the audio
///   rendering pipeline.
/// - **Text** tracks contain title/subtitle overlays rendered as text
///   composited on top of video tracks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrackType {
    /// A track carrying video frame sequences.
    Video,

    /// A track carrying audio sample sequences.
    Audio,

    /// A track carrying text overlay content (titles, subtitles, captions).
    Text,
}

/// The type of media a `MediaItem` represents.
///
/// When media files are imported into the project's media pool, each file
/// is classified based on its content. This classification determines which
/// tracks the media can be placed on and how it is processed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaType {
    /// A media file containing video frames (possibly with embedded audio).
    Video,

    /// A media file containing only audio samples.
    Audio,

    /// A still image file (e.g., PNG, JPEG, BMP).
    Image,
}

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// Configuration settings for a project that define output parameters.
///
/// `ProjectSettings` controls the target resolution, frame rate, audio sample
/// rate, color depth, and preview quality. These settings affect both the
/// preview rendering pipeline and the final export output.
///
/// # Default Values
///
/// The `Default` implementation provides sensible defaults for a typical
/// HD video project: 1920×1080 at 30 fps, 48 kHz audio, 8-bit color depth,
/// and medium preview quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// The output video width in pixels (e.g., 1920 for Full HD).
    pub resolution_width: u32,

    /// The output video height in pixels (e.g., 1080 for Full HD).
    pub resolution_height: u32,

    /// The output video frame rate in frames per second (e.g., 29.97, 30, 60).
    pub frame_rate: f64,

    /// The audio sample rate in Hz (e.g., 44100, 48000, 96000).
    pub sample_rate: u32,

    /// The color bit depth per channel (e.g., 8 for standard, 10 or 12 for HDR).
    pub color_depth: u32,

    /// The quality level for real-time preview rendering.
    ///
    /// This is a string identifier (e.g., "low", "medium", "high", "full")
    /// that controls how much processing is applied during preview playback.
    /// Lower quality previews reduce CPU/GPU load for smoother scrubbing.
    pub preview_quality: String,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            resolution_width: 1920,
            resolution_height: 1080,
            frame_rate: 30.0,
            sample_rate: 48000,
            color_depth: 8,
            preview_quality: "medium".to_string(),
        }
    }
}

/// An instance of a filter (effect) applied to a clip.
///
/// `FilterInstance` represents a specific application of a filter type to a
/// clip, with its own parameter values and enabled/disabled state. Multiple
/// filter instances of the same type can be applied to a single clip with
/// different parameters.
///
/// The `order` field determines the rendering sequence: filters with lower
/// `order` values are applied first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterInstance {
    /// A unique identifier for this filter instance.
    pub id: Uuid,

    /// The type identifier of the filter (e.g., "brightness", "contrast",
    /// "blur", "color_grade", "sharpen").
    pub filter_type: String,

    /// The filter's parameter values as a JSON object.
    ///
    /// The schema of this value depends on the `filter_type`. For example,
    /// a "brightness" filter might have `{"amount": 0.2}`, while a "blur"
    /// filter might have `{"radius": 5.0, "type": "gaussian"}`.
    pub params: serde_json::Value,

    /// Whether this filter is currently active and will be applied during
    /// rendering. Disabled filters are preserved in the project but skipped
    /// during playback and export.
    pub enabled: bool,

    /// The rendering order of this filter relative to other filters on the
    /// same clip. Lower values are applied first in the filter chain.
    pub order: u32,
}

/// A timestamped marker on the timeline for navigation and annotation.
///
/// Markers allow users to annotate specific points in time on the timeline
/// with a name and color. They are used for navigation, organization, and
/// collaboration (e.g., marking edit points, review comments, or chapter
/// boundaries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    /// A unique identifier for this marker.
    pub id: Uuid,

    /// A short descriptive label for the marker (e.g., "Chapter 1", "Cut point").
    pub name: String,

    /// The timestamp in seconds where the marker is placed on the timeline.
    pub timestamp: f64,

    /// The display color for the marker, represented as a hex string
    /// (e.g., "#FF0000" for red, "#00FF00" for green).
    pub color: String,
}

/// A clip placed on a track, referencing a media item.
///
/// `Clip` is the fundamental unit of content on the timeline. It references
/// a `MediaItem` via `media_id` and defines the time range of the source
/// media that is used (`in_point` to `out_point`), as well as the position
/// and duration on the timeline (`start_time` and `duration`).
///
/// Clips can also have applied filters, transitions, and a playback speed
/// multiplier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    /// A unique identifier for this clip.
    pub id: Uuid,

    /// The ID of the `MediaItem` this clip references as its source content.
    pub media_id: String,

    /// The ID of the `Track` this clip belongs to.
    pub track_id: String,

    /// The start time in seconds on the timeline where this clip begins.
    pub start_time: f64,

    /// The duration in seconds of this clip on the timeline.
    ///
    /// Note: this may differ from `out_point - in_point` when `speed` != 1.0.
    pub duration: f64,

    /// The in-point in seconds within the source media (start of the used range).
    pub in_point: f64,

    /// The out-point in seconds within the source media (end of the used range).
    pub out_point: f64,

    /// The ordered list of filter instances applied to this clip.
    ///
    /// Filters are applied in ascending `order` during rendering.
    pub filters: Vec<FilterInstance>,

    /// The names of transitions applied at the boundaries of this clip.
    ///
    /// Transitions define cross-fade or other effects at the junction
    /// between consecutive clips.
    pub transitions: Vec<String>,

    /// The playback speed multiplier (1.0 = normal, 0.5 = half speed, 2.0 = double).
    ///
    /// When speed != 1.0, the clip's effective source duration is
    /// `(out_point - in_point) / speed`, which maps to `duration` on the timeline.
    pub speed: f64,
}

/// A track on the timeline containing an ordered sequence of clips.
///
/// Tracks are horizontal lanes on the timeline that hold clips of a specific
/// type (video, audio, or text). Tracks can be locked (preventing edits),
/// hidden (preventing rendering), and have an audio volume level (for audio
/// tracks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// A unique identifier for this track.
    pub id: Uuid,

    /// The type of content this track carries (video, audio, or text).
    pub track_type: TrackType,

    /// The display name of the track (e.g., "Video 1", "Audio - Music").
    pub name: String,

    /// The ordered list of clips on this track, sorted by `start_time`.
    pub clips: Vec<Clip>,

    /// Whether this track is locked and cannot be modified by the user.
    /// Locked tracks prevent clip additions, removals, and rearranging.
    pub locked: bool,

    /// Whether this track is visible and will be rendered during playback
    /// and export. Hidden tracks are skipped entirely.
    pub visible: bool,

    /// The audio volume level for this track, ranging from 0.0 (muted) to
    /// 1.0 (full volume). Values above 1.0 amplify the audio signal.
    /// For video and text tracks, this field is ignored.
    pub volume: f64,
}

/// The timeline structure that holds all tracks and markers.
///
/// `Timeline` is the central compositional structure of a project. It contains
/// an ordered list of tracks (which hold clips) and a set of markers for
/// navigation and annotation. The `duration` field represents the total
/// length of the timeline in seconds, computed from the farthest-reaching clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    /// The ordered list of tracks on the timeline.
    ///
    /// Tracks are rendered from bottom to top for video (lower tracks are
    /// composited first, upper tracks overlay them), and mixed additively
    /// for audio.
    pub tracks: Vec<Track>,

    /// The total duration of the timeline in seconds.
    ///
    /// This is the maximum `start_time + duration` across all clips on all
    /// tracks, representing the length of the final output.
    pub duration: f64,

    /// The set of markers placed on the timeline for navigation.
    pub markers: Vec<Marker>,
}

impl Default for Timeline {
    /// Provides a default empty timeline with no tracks, zero duration, and no markers.
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            duration: 0.0,
            markers: Vec::new(),
        }
    }
}

/// A media file imported into the project's media pool.
///
/// `MediaItem` represents a source file (video, audio, or image) that has
/// been imported and analyzed. It stores metadata about the file's format,
/// dimensions, duration, codec, bitrate, and file size, as well as an
/// optional thumbnail path for preview display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    /// A unique identifier for this media item within the project.
    pub id: Uuid,

    /// The display name of the media file (typically the filename without extension).
    pub name: String,

    /// The absolute file path on the local filesystem.
    pub path: String,

    /// The type of media content (video, audio, or image).
    pub media_type: MediaType,

    /// The duration of the media in seconds. For images, this is typically 0.0
    /// (or a configured default still duration).
    pub duration: f64,

    /// The width in pixels of the video/image. For audio files, this is 0.
    pub width: u32,

    /// The height in pixels of the video/image. For audio files, this is 0.
    pub height: u32,

    /// The frame rate in fps of the video. For audio and image files, this is 0.0.
    pub frame_rate: f64,

    /// The codec name used to encode the media (e.g., "h264", "hevc", "aac", "png").
    pub codec: String,

    /// The bitrate in bits per second of the media stream.
    pub bitrate: u64,

    /// The file size in bytes on disk.
    pub file_size: u64,

    /// An optional path to a generated thumbnail image for display in the
    /// media pool UI. Thumbnails are typically generated during import.
    pub thumbnail_path: Option<String>,
}

/// A FlowCut project containing all editorial data.
///
/// `Project` is the top-level data structure that holds everything related to
/// an editing session: the timeline, the media pool, project settings, and
/// metadata (name, path, timestamps).
///
/// Projects are serialized to JSON files for persistence and deserialized
/// when opened. The `id` field uniquely identifies the project even when
/// copied or renamed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// A unique identifier for this project, generated on creation.
    pub id: Uuid,

    /// The human-readable name of the project (e.g., "Summer Vacation Edit").
    pub name: String,

    /// The absolute file path where this project is stored on disk.
    pub path: String,

    /// The timeline containing all tracks, clips, and markers.
    pub timeline: Timeline,

    /// The media pool of all imported source files available for use on the timeline.
    pub media_pool: Vec<MediaItem>,

    /// The timestamp when this project was first created.
    pub created_at: DateTime<Utc>,

    /// The timestamp when this project was last modified.
    /// Updated on every save or significant edit.
    pub modified_at: DateTime<Utc>,

    /// The project's output and preview configuration settings.
    pub settings: ProjectSettings,
}

/// A lightweight summary of a project for the "recent projects" list.
///
/// `ProjectInfo` contains only the essential metadata needed to display a
/// project in a recent projects list or a project browser, without the full
/// timeline and media pool data. This keeps the recent projects list fast
/// to load and small in memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// The unique identifier of the project (matches `Project::id`).
    pub id: Uuid,

    /// The display name of the project.
    pub name: String,

    /// The file path of the project on disk.
    pub path: String,

    /// The total duration of the project's timeline in seconds.
    pub duration: f64,

    /// The number of tracks in the project's timeline.
    pub track_count: usize,

    /// The number of clips across all tracks in the timeline.
    pub clip_count: usize,

    /// The timestamp when the project was created.
    pub created_at: DateTime<Utc>,

    /// The timestamp when the project was last modified.
    pub modified_at: DateTime<Utc>,
}

impl From<&Project> for ProjectInfo {
    /// Creates a `ProjectInfo` summary from a full `Project` reference.
    ///
    /// This conversion extracts the key metadata fields without cloning
    /// the heavy timeline and media pool data.
    fn from(project: &Project) -> Self {
        let track_count = project.timeline.tracks.len();
        let clip_count = project.timeline.tracks.iter().map(|t| t.clips.len()).sum();

        Self {
            id: project.id,
            name: project.name.clone(),
            path: project.path.clone(),
            duration: project.timeline.duration,
            track_count,
            clip_count,
            created_at: project.created_at,
            modified_at: project.modified_at,
        }
    }
}

/// The Tauri-managed state container for project management.
///
/// `ProjectState` holds the currently open project (if any) and the list of
/// recently accessed projects. It is registered as a Tauri managed state via
/// `app.manage(ProjectState::new())` and accessed by project-related commands.
///
/// # Thread Safety
///
/// All mutable fields are wrapped in `std::sync::Mutex` for safe concurrent
/// access from multiple Tauri command handlers.
///
/// # Lifecycle
///
/// - On application start, `current_project` is `None` (no project is open).
/// - When the user creates or opens a project, `current_project` is set.
/// - When the user closes a project, `current_project` is reset to `None`.
/// - The `recent_projects` list is updated whenever a project is opened or saved.
pub struct ProjectState {
    /// The currently open project, if any.
    ///
    /// This is `None` when no project is open (initial state or after closing
    /// a project). It is `Some(Project)` when a project has been created or
    /// opened.
    pub current_project: Mutex<Option<Project>>,

    /// A list of recently accessed projects for the "recent projects" UI.
    ///
    /// This list is capped at a maximum size (typically 10–20 entries) and
    /// is updated whenever a project is opened, saved, or closed. Older
    /// entries are evicted when the list exceeds the maximum size.
    pub recent_projects: Mutex<Vec<ProjectInfo>>,
}

impl ProjectState {
    /// Creates a new `ProjectState` with no open project and an empty recent list.
    ///
    /// # Returns
    ///
    /// A fresh `ProjectState` ready for Tauri state management.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flowcut_lib::project::ProjectState;
    ///
    /// let state = ProjectState::new();
    /// assert!(state.current_project.lock().unwrap().is_none());
    /// assert!(state.recent_projects.lock().unwrap().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            current_project: Mutex::new(None),
            recent_projects: Mutex::new(Vec::new()),
        }
    }

    /// Opens a project by setting it as the current project and adding it to
    /// the recent projects list.
    ///
    /// # Arguments
    ///
    /// * `project` - The `Project` to open.
    ///
    /// # Behavior
    ///
    /// - Sets `current_project` to `Some(project)`.
    /// - Adds a `ProjectInfo` summary to `recent_projects`.
    /// - If the project was already in the recent list, it is moved to the top.
    pub fn open_project(&self, project: Project) {
        let info = ProjectInfo::from(&project);
        *self.current_project.lock().unwrap() = Some(project);

        let mut recent = self.recent_projects.lock().unwrap();
        // Remove duplicate entry if it exists
        recent.retain(|p| p.id != info.id);
        // Insert at the top
        recent.insert(0, info);
        // Cap the list at 20 entries
        recent.truncate(20);
    }

    /// Closes the current project by setting `current_project` to `None`.
    ///
    /// This does not remove the project from the recent projects list, so
    /// the user can quickly reopen it.
    pub fn close_project(&self) {
        *self.current_project.lock().unwrap() = None;
    }

    /// Returns a reference to the currently open project, if any.
    ///
    /// This method clones the project to avoid holding the Mutex lock across
    /// a serialization boundary. For read-only checks (e.g., "is a project
    /// open?"), prefer checking `current_project.lock().unwrap().is_some()`.
    ///
    /// # Returns
    ///
    /// `Some(Project)` if a project is open, `None` otherwise.
    pub fn get_current_project(&self) -> Option<Project> {
        self.current_project.lock().unwrap().clone()
    }

    /// Returns a cloned copy of the recent projects list.
    ///
    /// # Returns
    ///
    /// A `Vec<ProjectInfo>` of recently accessed projects, ordered most-recent-first.
    pub fn get_recent_projects(&self) -> Vec<ProjectInfo> {
        self.recent_projects.lock().unwrap().clone()
    }

    /// Updates the current project in place (e.g., after an edit or save).
    ///
    /// This method replaces the current project with the given updated version
    /// and also updates its entry in the recent projects list.
    ///
    /// # Arguments
    ///
    /// * `project` - The updated `Project` to store.
    ///
    /// # Panics
    ///
    /// Panics if no project is currently open (callers should verify first).
    pub fn update_project(&self, project: Project) {
        let info = ProjectInfo::from(&project);
        *self.current_project.lock().unwrap() = Some(project);

        let mut recent = self.recent_projects.lock().unwrap();
        // Update the matching entry in the recent list
        if let Some(entry) = recent.iter_mut().find(|p| p.id == info.id) {
            *entry = info;
        }
    }
}

impl Default for ProjectState {
    /// Provides a default `ProjectState` equivalent to `ProjectState::new()`.
    fn default() -> Self {
        Self::new()
    }
}
