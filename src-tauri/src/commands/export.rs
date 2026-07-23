//! Export commands for FlowCut.
//!
//! This module provides Tauri command handlers for exporting the
//! edited timeline to a final video file. The export system runs
//! asynchronously — a `start_export` command initiates a background
//! job, and the frontend can poll `get_export_progress` to track
//! completion. Jobs can be cancelled at any time before they finish.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::engine::EngineState;
use crate::export::{ExportConfig as InternalExportConfig, ExportJob, ExportState, QualityPreset};
use crate::project::ProjectState;

/// Configuration for an export job.
///
/// Encapsulates all the parameters the user specifies when exporting
/// their project: output path, format, codec, resolution, quality,
/// and other encoding settings.
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
    /// "slow", "veryslow").
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
    /// Custom FFmpeg encoding flags.
    pub custom_flags: Option<String>,
}

/// Progress information for an active export job.
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
    /// Estimated time remaining in seconds.
    pub estimated_time_remaining: f64,
    /// Current encoding speed in frames per second.
    pub current_speed_fps: f64,
    /// Output file size so far in bytes.
    pub output_file_size: u64,
    /// Whether the export job has completed.
    pub completed: bool,
    /// Whether the export job was cancelled.
    pub cancelled: bool,
    /// Error message if the export failed.
    pub error: Option<String>,
}

/// Describes an available export format and its capabilities.
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
    /// Maximum supported resolution as a string.
    pub max_resolution: String,
    /// Typical file extension.
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

/// Converts the command-level ExportConfig to the internal ExportConfig.
fn to_internal_config(config: ExportConfig) -> InternalExportConfig {
    let quality_preset = match config.preset.as_str() {
        "ultrafast" => QualityPreset::UltraFast,
        "fast" => QualityPreset::Fast,
        "veryslow" => QualityPreset::VerySlow,
        "slow" => QualityPreset::Slow,
        _ => QualityPreset::Medium,
    };

    InternalExportConfig {
        format: config.format,
        codec: config.video_codec,
        resolution_width: config.width,
        resolution_height: config.height,
        frame_rate: config.frame_rate,
        bitrate: (config.video_bitrate as u64) * 1000, // kbps to bps
        audio_codec: config.audio_codec,
        audio_bitrate: (config.audio_bitrate as u64) * 1000, // kbps to bps
        output_path: config.output_path,
        quality_preset,
    }
}

/// Converts an internal ExportJob to the command-level ExportProgress.
fn job_to_progress(job: &ExportJob) -> ExportProgress {
    let phase = match job.status {
        crate::export::ExportStatus::Pending => "pending",
        crate::export::ExportStatus::Processing => "processing",
        crate::export::ExportStatus::Completed => "completed",
        crate::export::ExportStatus::Failed => "failed",
        crate::export::ExportStatus::Cancelled => "cancelled",
    };

    let (completed, cancelled, error) = match job.status {
        crate::export::ExportStatus::Completed => (true, false, None),
        crate::export::ExportStatus::Failed => (true, false, Some("Export failed".to_string())),
        crate::export::ExportStatus::Cancelled => (true, true, None),
        _ => (false, false, None),
    };

    ExportProgress {
        job_id: job.id.to_string(),
        phase: phase.to_string(),
        progress: job.progress.percent,
        frames_processed: job.progress.current_frame,
        total_frames: job.progress.total_frames,
        estimated_time_remaining: job.progress.estimated_remaining_seconds,
        current_speed_fps: job.progress.current_fps,
        output_file_size: 0,
        completed,
        cancelled,
        error,
    }
}

/// Starts an asynchronous export job.
///
/// Initiates the export pipeline with the given configuration and
/// returns a unique job ID that the frontend can use to track progress.
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

    // Verify the project is open
    let current_project = project_state.get_current_project();
    if current_project.is_none() {
        return Err(ExportError {
            kind: "no_active_project".into(),
            message: "No active project. Open a project before exporting.".into(),
        });
    }

    // Verify the engine is initialized
    let is_initialized = *engine_state.is_initialized.lock().unwrap();
    if !is_initialized {
        return Err(ExportError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized. Call initialize_engine first."
                .into(),
        });
    }

    // Check if an export job is already running
    let active_jobs = export_state.get_active_jobs();
    if !active_jobs.is_empty() {
        return Err(ExportError {
            kind: "job_already_active".into(),
            message: "An export job is already running. Cancel it before starting a new one."
                .into(),
        });
    }

    // Start the export job using the ExportState method
    let internal_config = to_internal_config(config);
    let job_id = export_state.create_job(internal_config);

    log::info!("Export job started: {}", job_id);
    Ok(job_id.to_string())
}

/// Retrieves the progress of an active export job.
///
/// Returns a snapshot of the export job's current state.
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

    // Parse the job_id string to Uuid
    let parsed_id = uuid::Uuid::parse_str(&job_id).map_err(|e| ExportError {
        kind: "invalid_job_id".into(),
        message: format!("Invalid job ID format '{}': {}", job_id, e),
    })?;

    // Look up the job using ExportState's method
    let job = export_state.get_job(parsed_id);

    if job.is_none() {
        return Err(ExportError {
            kind: "job_not_found".into(),
            message: format!("Export job '{}' not found.", job_id),
        });
    }

    let job = job.unwrap();
    Ok(job_to_progress(&job))
}

/// Cancels an active export job.
///
/// Signals the export pipeline to stop processing and clean up.
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

    // Parse the job_id string to Uuid
    let parsed_id = uuid::Uuid::parse_str(&job_id).map_err(|e| ExportError {
        kind: "invalid_job_id".into(),
        message: format!("Invalid job ID format '{}': {}", job_id, e),
    })?;

    // Cancel the job using ExportState's method
    let success = export_state.cancel_job(parsed_id);

    if !success {
        return Err(ExportError {
            kind: "cancel_failed".into(),
            message: format!("Export job '{}' not found or already completed.", job_id),
        });
    }

    log::info!("Export job '{}' cancellation signal sent", job_id);
    Ok(true)
}

/// Retrieves the list of supported export formats.
///
/// Returns all container formats and their associated codec options
/// that the current FFmpeg engine build supports.
#[tauri::command]
pub fn get_export_formats(
    engine_state: State<EngineState>,
) -> Result<Vec<ExportFormat>, ExportError> {
    log::info!("Retrieving available export formats");

    // Check engine is initialized
    let is_initialized = *engine_state.is_initialized.lock().unwrap();
    if !is_initialized {
        return Err(ExportError {
            kind: "engine_not_initialized".into(),
            message: "The video engine has not been initialized.".into(),
        });
    }

    // Get the static export formats from ExportState
    let internal_formats = ExportState::get_export_formats();

    // Map internal formats to the command-level format
    let export_formats: Vec<ExportFormat> = internal_formats
        .into_iter()
        .map(|f| ExportFormat {
            format: f.extension.clone(),
            name: f.name,
            description: f.description,
            video_codecs: f.codecs.clone(),
            audio_codecs: vec!["aac".to_string(), "opus".to_string(), "mp3".to_string()],
            max_resolution: if f.supported_resolutions.is_empty() {
                "1920x1080".to_string()
            } else {
                let (w, h) = f.supported_resolutions[0];
                format!("{}x{}", w, h)
            },
            extension: f.extension,
        })
        .collect();

    log::info!("Found {} export formats", export_formats.len());
    Ok(export_formats)
}
