//! Export commands for FlowCut.
//!
//! This module provides Tauri command handlers for exporting the
//! edited timeline to a final video file. The export system runs
//! asynchronously — a `start_export` command initiates a background
//! job, and the frontend can poll `get_export_progress` to track
//! completion. Jobs can be cancelled at any time before they finish.
//!
//! # Export Pipeline
//!
//! The export pipeline composites all tracks, applies effects and
//! transitions, encodes the result using FFmpeg, and writes the
//! output file to the user-specified path. The pipeline runs on
//! a background thread to avoid blocking the UI.
//!
//! # State Dependencies
//!
//! Commands depend on [`ExportState`] for managing export jobs,
//! [`ProjectState`] for the timeline data, and [`EngineState`]
//! for FFmpeg encoding.

use serde::{Deserialize, Serialize};
use tauri::State;

use flowcut_lib::engine::EngineState;
use flowcut_lib::export::ExportState;
use flowcut_lib::project::ProjectState;

/// Configuration for an export job.
///
/// Encapsulates all the parameters the user specifies when exporting
/// their project: output path, format, codec, resolution, quality,
/// and other encoding settings. The frontend should construct this
/// struct from the export settings panel.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportConfig {
    /// Absolute filesystem path for the output file (including extension).
    pub output_path: String,
    /// Container format (e.g. "mp4", "mov", "mkv", "webm", "avi").
    pub format: String,
    /// Video codec (e.g. "h264", "hevc", "vp9", "prores", "av1").
    pub video_codec: String,
    /// Audio codec (e.g. "aac", "opus", "mp3", "pcm_s16le", "flac").
    pub audio_codec: String,
    /// Output video width in pixels.
    pub width: u32,
    /// Output video height in pixels.
    pub height: u32,
    /// Video frame rate in frames per second.
    pub frame_rate: f64,
    /// Video bitrate in kbps. 0 means use the codec's default.
    pub video_bitrate: u32,
    /// Audio bitrate in kbps. 0 means use the codec's default.
    pub audio_bitrate: u32,
    /// Video quality preset (e.g. "ultrafast", "fast", "medium",
    /// "slow", "veryslow"). Lower presets produce better quality
    /// but take longer.
    pub preset: String,
    /// Encoding profile for H.264/HEVC (e.g. "baseline", "main",
    /// "high", "high10").
    pub profile: String,
    /// Number of audio channels in the output (1 = mono, 2 = stereo).
    pub audio_channels: u32,
    /// Audio sample rate in Hz (e.g. 44100, 48000).
    pub audio_sample_rate: u32,
    /// Whether to include a custom title metadata tag.
    pub metadata_title: Option<String>,
    /// Whether to include a custom comment metadata tag.
    pub metadata_comment: Option<String>,
    /// Whether to embed the project's creation date as metadata.
    pub embed_creation_date: bool,
    /// Custom FFmpeg encoding flags, passed directly to the encoder.
    /// Use with caution; invalid flags may cause the export to fail.
    pub custom_flags: Option<String>,
}

/// Progress information for an active export job.
///
/// Returned by [`get_export_progress`] to allow the frontend to
/// display a progress bar and estimated time remaining.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportProgress {
    /// The unique job ID this progress belongs to.
    pub job_id: String,
    /// Current phase of the export pipeline.
    pub phase: String,
    /// Overall completion percentage (0.0–100.0).
    pub progress: f64,
    /// Number of frames processed so far.
    pub frames_processed: u64,
    /// Total number of frames to process.
    pub total_frames: u64,
    /// Estimated time remaining in seconds, based on current speed.
    pub estimated_time_remaining: f64,
    /// Current encoding speed in frames per second.
    pub current_speed_fps: f64,
    /// Output file size so far in bytes.
    pub output_file_size: u64,
    /// Whether the export job has completed (successfully or with error).
    pub completed: bool,
    /// Whether the export job was cancelled by the user.
    pub cancelled: bool,
    /// Error message if the export failed, `None` if successful.
    pub error: Option<String>,
}

/// Describes an available export format and its capabilities.
///
/// Returned by [`get_export_formats`] to populate the frontend's
/// format selection dropdown with supported options.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportFormat {
    /// Container format identifier (e.g. "mp4", "mov").
    pub format: String,
    /// Human-readable name (e.g. "MP4 (H.264)", "MOV (ProRes)").
    pub name: String,
    /// Brief description of the format's typical use case.
    pub description: String,
    /// Supported video codecs for this container.
    pub video_codecs: Vec<String>,
    /// Supported audio codecs for this container.
    pub audio_codecs: Vec<String>,
    /// Maximum supported resolution as a string (e.g. "3840x2160").
    pub max_resolution: String,
    /// Typical file extension (e.g. ".mp4", ".mov").
    pub extension: String,
}

/// Error type for export command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Starts an asynchronous export job.
///
/// Initiates the export pipeline with the given configuration and
/// returns a unique job ID that the frontend can use to track
/// progress. The export runs on a background thread and does not
/// block the UI.
///
/// # Parameters
///
/// - `config` — An [`ExportConfig`] struct specifying the output
///   file path, codec, resolution, and all encoding parameters.
///
/// # Returns
///
/// A unique string identifier (UUID v4) for the export job. The
/// frontend should store this ID and use it in subsequent calls
/// to [`get_export_progress`] and [`cancel_export`].
///
/// # Workflow
///
/// ```text
/// start_export(config) → job_id
/// while !completed:
///     get_export_progress(job_id) → ExportProgress
///     update UI progress bar
/// if error: show error to user
/// else: notify export complete
/// ```
///
/// # Error Cases
///
/// Returns an [`ExportError`] if:
/// - There is no active project.
/// - The output path is invalid or not writable.
/// - The specified codec/format combination is unsupported.
/// - An export job is already running for this project.
#[tauri::command]
pub fn start_export(
    config: ExportConfig,
    export_state: State<ExportState>,
    project_state: State<ProjectState>,
    engine_state: State<EngineState>,
) -> Result<String, ExportError> {
    log::info!(
        "Starting export: path={}, format={}, codec={}",
        config.output_path,
        config.format,
        config.video_codec
    );

    // Validate essential config fields
    if config.output_path.trim().is_empty() {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Output path must not be empty.".into(),
        });
    }
    if config.format.trim().is_empty() {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Export format must not be empty.".into(),
        });
    }
    if config.video_codec.trim().is_empty() {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Video codec must not be empty.".into(),
        });
    }
    if config.width == 0 || config.height == 0 {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Output resolution must not be zero.".into(),
        });
    }
    if config.frame_rate <= 0.0 {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Frame rate must be positive.".into(),
        });
    }

    // Acquire locks on all required state
    let mut export = export_state.data.lock().map_err(|e| ExportError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire export state lock: {}", e),
    })?;

    let project = project_state.data.lock().map_err(|e| ExportError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire project state lock: {}", e),
    })?;

    let engine = engine_state.data.lock().map_err(|e| ExportError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    // Verify the project is open
    if !project.is_open() {
        return Err(ExportError {
            kind: "no_active_project".into(),
            message: "No active project. Open a project before exporting.".into(),
        });
    }

    // Verify the engine is initialized
    if !engine.is_initialized() {
        return Err(ExportError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized. Call initialize_engine first."
                .into(),
        });
    }

    // Verify the codec/format combination is supported
    engine
        .validate_export_config(&config.format, &config.video_codec, &config.audio_codec)
        .map_err(|e| ExportError {
            kind: "invalid_config".into(),
            message: format!("Unsupported export configuration: {}", e),
        })?;

    // Check if an export job is already running
    if export.has_active_job() {
        return Err(ExportError {
            kind: "job_already_active".into(),
            message: "An export job is already running. Cancel it before starting a new one."
                .into(),
        });
    }

    // Start the export job
    let job_id = export
        .start_job(config, &project, &engine)
        .map_err(|e| ExportError {
            kind: "start_failed".into(),
            message: format!("Failed to start export job: {}", e),
        })?;

    log::info!("Export job started: {}", job_id);
    Ok(job_id)
}

/// Retrieves the progress of an active export job.
///
/// Returns a snapshot of the export job's current state, including
/// the completion percentage, frames processed, estimated time
/// remaining, and any error information. The frontend should poll
/// this command periodically (e.g. every 500ms) while an export
/// is running.
///
/// # Parameters
///
/// - `job_id` — The UUID of the export job to query, as returned
///   by [`start_export`].
///
/// # Returns
///
/// An [`ExportProgress`] struct with the current job status, or an
/// [`ExportError`] if the job ID does not exist.
#[tauri::command]
pub fn get_export_progress(
    job_id: String,
    export_state: State<ExportState>,
) -> Result<ExportProgress, ExportError> {
    log::info!("Getting export progress for job: {}", job_id);

    if job_id.trim().is_empty() {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Job ID must not be empty.".into(),
        });
    }

    let export = export_state.data.lock().map_err(|e| ExportError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire export state lock: {}", e),
    })?;

    let progress = export.get_progress(&job_id).map_err(|e| ExportError {
        kind: "job_not_found".into(),
        message: format!("Export job '{}' not found: {}", job_id, e),
    })?;

    Ok(ExportProgress {
        job_id: progress.job_id,
        phase: progress.phase,
        progress: progress.progress,
        frames_processed: progress.frames_processed,
        total_frames: progress.total_frames,
        estimated_time_remaining: progress.estimated_time_remaining,
        current_speed_fps: progress.current_speed_fps,
        output_file_size: progress.output_file_size,
        completed: progress.completed,
        cancelled: progress.cancelled,
        error: progress.error,
    })
}

/// Cancels an active export job.
///
/// Signals the export pipeline to stop processing and clean up
/// any temporary files. The output file (if partially written)
/// is deleted. This command returns immediately, but the actual
/// cancellation may take a moment to propagate through the
/// encoding pipeline.
///
/// # Parameters
///
/// - `job_id` — The UUID of the export job to cancel.
///
/// # Returns
///
/// `true` if the cancellation signal was sent successfully, or an
/// [`ExportError`] if the job ID does not exist or the job has
/// already completed.
///
/// # Note
///
/// After cancellation, calling [`get_export_progress`] will show
/// `cancelled: true` once the pipeline has fully terminated.
#[tauri::command]
pub fn cancel_export(
    job_id: String,
    export_state: State<ExportState>,
) -> Result<bool, ExportError> {
    log::info!("Cancelling export job: {}", job_id);

    if job_id.trim().is_empty() {
        return Err(ExportError {
            kind: "validation".into(),
            message: "Job ID must not be empty.".into(),
        });
    }

    let mut export = export_state.data.lock().map_err(|e| ExportError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire export state lock: {}", e),
    })?;

    export.cancel_job(&job_id).map_err(|e| ExportError {
        kind: "cancel_failed".into(),
        message: format!("Failed to cancel export job '{}': {}", job_id, e),
    })?;

    log::info!("Export job '{}' cancellation signal sent", job_id);
    Ok(true)
}

/// Retrieves the list of supported export formats.
///
/// Returns all container formats and their associated codec options
/// that the current FFmpeg engine build supports. The frontend uses
/// this to populate the export format dropdown and to validate the
/// user's codec selections.
///
/// # Returns
///
/// A vector of [`ExportFormat`] structs, one per supported container.
/// The list is determined by the FFmpeg libraries available on the
/// current system.
#[tauri::command]
pub fn get_export_formats(
    engine_state: State<EngineState>,
) -> Result<Vec<ExportFormat>, ExportError> {
    log::info!("Retrieving available export formats");

    let engine = engine_state.data.lock().map_err(|e| ExportError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    if !engine.is_initialized() {
        return Err(ExportError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized.".into(),
        });
    }

    let formats = engine.get_export_formats().map_err(|e| ExportError {
        kind: "formats_query_failed".into(),
        message: format!("Failed to query export formats: {}", e),
    })?;

    let export_formats = formats
        .into_iter()
        .map(|f| ExportFormat {
            format: f.format,
            name: f.name,
            description: f.description,
            video_codecs: f.video_codecs,
            audio_codecs: f.audio_codecs,
            max_resolution: f.max_resolution,
            extension: f.extension,
        })
        .collect();

    log::info!("Found {} export formats", export_formats.len());
    Ok(export_formats)
}
