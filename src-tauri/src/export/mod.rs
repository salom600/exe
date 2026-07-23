//! Export management state module for FlowCut.
//!
//! This module defines all data structures related to video export, including
//! export job tracking, configuration, progress reporting, and format metadata.
//! The `ExportState` struct serves as the Tauri-managed state container that
//! tracks active and completed export jobs.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The current status of an export job in the processing pipeline.
///
/// Export jobs transition through these states during their lifecycle:
///
/// ```text
/// Pending → Processing → Completed
///                    ↘ Failed
///                    ↘ Cancelled
/// ```
///
/// - **Pending**: The job has been created but is waiting for processing to begin.
/// - **Processing**: The job is actively encoding frames and writing output.
/// - **Completed**: The job has finished successfully and the output file is ready.
/// - **Failed**: The job encountered an error and was aborted.
/// - **Cancelled**: The user manually cancelled the job before completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExportStatus {
    /// The job is queued and waiting for processing resources to become available.
    Pending,

    /// The job is actively processing (encoding frames, mixing audio, writing output).
    Processing,

    /// The job has completed successfully and the output file is available.
    Completed,

    /// The job failed due to an error (codec unavailable, disk full, etc.).
    Failed,

    /// The job was manually cancelled by the user before reaching completion.
    Cancelled,
}

/// Quality presets that balance encoding speed against output quality.
///
/// These presets correspond to FFmpeg's encoding speed presets and control
/// how much computational effort the encoder spends optimizing each frame.
/// Slower presets produce better compression (higher quality at the same bitrate
/// or lower bitrate at the same quality) but take significantly longer to encode.
///
/// # Trade-off Summary
///
/// | Preset      | Speed  | Quality | Compression |
/// |-------------|--------|---------|-------------|
/// | UltraFast   | ★★★★★ | ★★      | ★★          |
/// | Fast        | ★★★★  | ★★★     | ★★★         |
/// | Medium      | ★★★   | ★★★★   | ★★★★       |
/// | Slow        | ★★    | ★★★★★ | ★★★★★      |
/// | VerySlow    | ★     | ★★★★★ | ★★★★★+     |
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityPreset {
    /// Maximum encoding speed, lowest quality. Useful for quick previews or drafts.
    UltraFast,

    /// Fast encoding with acceptable quality. Good for iterative test exports.
    Fast,

    /// Balanced speed and quality. The recommended default for most exports.
    Medium,

    /// Slower encoding with high quality. Good for final production exports.
    Slow,

    /// Maximum quality, minimum speed. Best for archival or distribution masters.
    VerySlow,
}

impl QualityPreset {
    /// Returns the FFmpeg-compatible preset string for this quality level.
    ///
    /// These strings can be passed directly to FFmpeg's `-preset` option
    /// for codecs that support it (e.g., libx264, libx265).
    ///
    /// # Examples
    ///
    /// ```
    /// use flowcut_lib::export::QualityPreset;
    ///
    /// assert_eq!(QualityPreset::Medium.to_ffmpeg_string(), "medium");
    /// assert_eq!(QualityPreset::UltraFast.to_ffmpeg_string(), "ultrafast");
    /// ```
    pub fn to_ffmpeg_string(&self) -> &'static str {
        match self {
            QualityPreset::UltraFast => "ultrafast",
            QualityPreset::Fast => "fast",
            QualityPreset::Medium => "medium",
            QualityPreset::Slow => "slow",
            QualityPreset::VerySlow => "veryslow",
        }
    }

    /// Returns a human-readable description of this quality preset.
    ///
    /// Useful for displaying in the export configuration UI.
    pub fn description(&self) -> &'static str {
        match self {
            QualityPreset::UltraFast => {
                "Fastest encoding, lower quality — ideal for quick previews"
            }
            QualityPreset::Fast => "Fast encoding, good quality — ideal for test exports",
            QualityPreset::Medium => "Balanced speed and quality — recommended for most exports",
            QualityPreset::Slow => "Slower encoding, high quality — ideal for production exports",
            QualityPreset::VerySlow => {
                "Slowest encoding, best quality — ideal for archival masters"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Core data structures
// ---------------------------------------------------------------------------

/// Configuration for an export job defining all output parameters.
///
/// `ExportConfig` specifies exactly how the project's timeline should be
/// rendered and written to an output file, including format, codec, resolution,
/// frame rate, bitrate, audio settings, output path, and quality preset.
///
/// When an export job is started, this configuration is passed to the FFmpeg
/// encoding pipeline to construct the appropriate output parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// The container format for the output file (e.g., "mp4", "mkv", "mov", "webm").
    pub format: String,

    /// The video codec to use for encoding (e.g., "h264", "hevc", "vp9", "av1").
    pub codec: String,

    /// The output video width in pixels.
    pub resolution_width: u32,

    /// The output video height in pixels.
    pub resolution_height: u32,

    /// The output video frame rate in frames per second.
    pub frame_rate: f64,

    /// The target video bitrate in bits per second.
    ///
    /// A higher bitrate produces better visual quality but larger files.
    /// Typical values: 5_000_000 (5 Mbps) for HD, 20_000_000 (20 Mbps) for 4K.
    pub bitrate: u64,

    /// The audio codec to use for encoding (e.g., "aac", "opus", "mp3", "flac").
    pub audio_codec: String,

    /// The target audio bitrate in bits per second.
    ///
    /// Typical values: 128_000 (128 kbps) for AAC, 320_000 (320 kbps) for high quality.
    pub audio_bitrate: u64,

    /// The absolute file path where the output file will be written.
    pub output_path: String,

    /// The quality preset controlling the speed/quality trade-off during encoding.
    pub quality_preset: QualityPreset,
}

impl Default for ExportConfig {
    /// Provides sensible default export configuration for a standard MP4/H.264 export.
    fn default() -> Self {
        Self {
            format: "mp4".to_string(),
            codec: "h264".to_string(),
            resolution_width: 1920,
            resolution_height: 1080,
            frame_rate: 30.0,
            bitrate: 8_000_000,
            audio_codec: "aac".to_string(),
            audio_bitrate: 192_000,
            output_path: String::new(),
            quality_preset: QualityPreset::Medium,
        }
    }
}

/// Real-time progress information for an active export job.
///
/// `ExportProgress` is continuously updated during the encoding process and
/// can be polled by the frontend via the `get_export_progress` command to
/// display progress bars, estimated times, and current encoding speed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    /// The percentage of completion from 0.0 to 100.0.
    pub percent: f64,

    /// The current frame number being encoded.
    pub current_frame: u64,

    /// The total number of frames to encode.
    pub total_frames: u64,

    /// The number of seconds elapsed since the export started.
    pub elapsed_seconds: f64,

    /// The estimated number of seconds remaining until completion.
    ///
    /// Calculated from the current encoding speed and remaining frames.
    pub estimated_remaining_seconds: f64,

    /// The current encoding speed in frames per second.
    ///
    /// Useful for showing the user how fast the export is progressing.
    pub current_fps: f64,
}

impl Default for ExportProgress {
    /// Provides a default progress state at 0% completion.
    fn default() -> Self {
        Self {
            percent: 0.0,
            current_frame: 0,
            total_frames: 0,
            elapsed_seconds: 0.0,
            estimated_remaining_seconds: 0.0,
            current_fps: 0.0,
        }
    }
}

/// A tracked export job with its configuration, progress, and status.
///
/// `ExportJob` represents the full lifecycle of an export operation from
/// creation through completion (or failure/cancellation). It is stored in
/// `ExportState` and can be queried by the frontend at any time.
///
/// # Lifecycle
///
/// 1. Created with `ExportStatus::Pending` and default `ExportProgress`.
/// 2. Transitioned to `ExportStatus::Processing` when encoding begins.
/// 3. Progress is continuously updated during processing.
/// 4. Final transition to `Completed`, `Failed`, or `Cancelled`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    /// A unique identifier for this export job.
    pub id: Uuid,

    /// The configuration defining what and how to export.
    pub config: ExportConfig,

    /// The current progress of the export (updated during processing).
    pub progress: ExportProgress,

    /// The current status of the export job in the processing pipeline.
    pub status: ExportStatus,
}

/// Metadata about an export format supported by the engine.
///
/// `ExportFormat` describes a container format (e.g., MP4, MKV) along with
/// the codecs that can be used within it and the resolutions it typically
/// supports. This information is used by the frontend to populate the export
/// configuration UI with valid options.
///
/// # Examples
///
/// A typical MP4 format definition:
///
/// ```text
/// name:                "MP4 (MPEG-4 Part 14)"
/// extension:           "mp4"
/// description:         "Universal format supported by most devices and platforms"
/// codecs:              ["h264", "hevc", "av1"]
/// supported_resolutions: [(1920, 1080), (3840, 2160), (1280, 720)]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFormat {
    /// The human-readable name of the format (e.g., "MP4 (MPEG-4 Part 14)").
    pub name: String,

    /// The common file extension for this format (e.g., "mp4", "mkv", "mov").
    pub extension: String,

    /// A brief description of the format's characteristics and typical use cases.
    pub description: String,

    /// The list of video codecs that can be used within this container format.
    pub codecs: Vec<String>,

    /// The list of standard resolutions supported for this format, expressed
    /// as (width, height) pairs in pixels.
    pub supported_resolutions: Vec<(u32, u32)>,
}

// ---------------------------------------------------------------------------
// State management
// ---------------------------------------------------------------------------

/// The Tauri-managed state container for export job management.
///
/// `ExportState` tracks all export jobs — both currently active and previously
/// completed — and provides methods for creating, querying, and cancelling
/// export jobs. It is registered as a Tauri managed state via
/// `app.manage(ExportState::new())`.
///
/// # Thread Safety
///
/// All mutable fields are wrapped in `std::sync::Mutex` for safe concurrent
/// access from multiple Tauri command handlers.
///
/// # Job Management
///
/// - **Active jobs**: Jobs currently in `Pending` or `Processing` status.
/// - **Completed jobs**: Jobs that have reached `Completed`, `Failed`, or
///   `Cancelled` status, kept for history display.
pub struct ExportState {
    /// The list of currently active export jobs (Pending or Processing).
    ///
    /// Active jobs are continuously updated during the encoding process.
    /// When a job finishes (or fails/is cancelled), it is moved from this
    /// list to `completed_jobs`.
    pub active_jobs: Mutex<Vec<ExportJob>>,

    /// The list of completed, failed, or cancelled export jobs.
    ///
    /// These jobs are retained for the export history UI, showing the user
    /// their recent export activity. Old entries may be pruned to limit
    /// memory usage.
    pub completed_jobs: Mutex<Vec<ExportJob>>,
}

impl ExportState {
    /// Creates a new `ExportState` with no active or completed jobs.
    ///
    /// # Returns
    ///
    /// A fresh `ExportState` ready for Tauri state management.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flowcut_lib::export::ExportState;
    ///
    /// let state = ExportState::new();
    /// assert!(state.active_jobs.lock().unwrap().is_empty());
    /// assert!(state.completed_jobs.lock().unwrap().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            active_jobs: Mutex::new(Vec::new()),
            completed_jobs: Mutex::new(Vec::new()),
        }
    }

    /// Creates a new export job with the given configuration and adds it to
    /// the active jobs list.
    ///
    /// The new job starts with `ExportStatus::Pending` and default
    /// `ExportProgress` (0% completion).
    ///
    /// # Arguments
    ///
    /// * `config` - The `ExportConfig` specifying the export parameters.
    ///
    /// # Returns
    ///
    /// The `Uuid` of the newly created export job, which can be used to
    /// query progress or cancel the job.
    pub fn create_job(&self, config: ExportConfig) -> Uuid {
        let job_id = Uuid::new_v4();
        let job = ExportJob {
            id: job_id,
            config,
            progress: ExportProgress::default(),
            status: ExportStatus::Pending,
        };

        self.active_jobs.lock().unwrap().push(job);
        job_id
    }

    /// Retrieves the progress and status of a specific export job.
    ///
    /// Searches both active and completed job lists for the given job ID.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The `Uuid` of the export job to query.
    ///
    /// # Returns
    ///
    /// `Some(ExportJob)` if the job was found, `None` if no job matches the ID.
    pub fn get_job(&self, job_id: Uuid) -> Option<ExportJob> {
        let active = self.active_jobs.lock().unwrap();
        if let Some(job) = active.iter().find(|j| j.id == job_id) {
            return Some(job.clone());
        }

        let completed = self.completed_jobs.lock().unwrap();
        completed.iter().find(|j| j.id == job_id).cloned()
    }

    /// Cancels an active export job by setting its status to `Cancelled` and
    /// moving it to the completed jobs list.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The `Uuid` of the export job to cancel.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and cancelled, `false` if no active job
    /// matches the ID.
    pub fn cancel_job(&self, job_id: Uuid) -> bool {
        let mut active = self.active_jobs.lock().unwrap();
        let job_index = active.iter().position(|j| j.id == job_id);

        if let Some(index) = job_index {
            let mut job = active.remove(index);
            job.status = ExportStatus::Cancelled;

            self.completed_jobs.lock().unwrap().push(job);
            return true;
        }

        false
    }

    /// Updates the progress of an active export job.
    ///
    /// This method is called periodically during the encoding process to
    /// report current frame count, percentage, elapsed time, and encoding speed.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The `Uuid` of the export job to update.
    /// * `progress` - The new `ExportProgress` snapshot.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and updated, `false` otherwise.
    pub fn update_progress(&self, job_id: Uuid, progress: ExportProgress) -> bool {
        let mut active = self.active_jobs.lock().unwrap();
        if let Some(job) = active.iter_mut().find(|j| j.id == job_id) {
            job.progress = progress;
            return true;
        }
        false
    }

    /// Marks an active export job as completed and moves it to the completed
    /// jobs list.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The `Uuid` of the export job to complete.
    /// * `final_progress` - The final `ExportProgress` at completion time
    ///   (typically percent = 100.0).
    ///
    /// # Returns
    ///
    /// `true` if the job was found and completed, `false` otherwise.
    pub fn complete_job(&self, job_id: Uuid, final_progress: ExportProgress) -> bool {
        let mut active = self.active_jobs.lock().unwrap();
        let job_index = active.iter().position(|j| j.id == job_id);

        if let Some(index) = job_index {
            let mut job = active.remove(index);
            job.status = ExportStatus::Completed;
            job.progress = final_progress;

            self.completed_jobs.lock().unwrap().push(job);
            return true;
        }

        false
    }

    /// Marks an active export job as failed and moves it to the completed
    /// jobs list.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The `Uuid` of the export job that failed.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and marked as failed, `false` otherwise.
    pub fn fail_job(&self, job_id: Uuid) -> bool {
        let mut active = self.active_jobs.lock().unwrap();
        let job_index = active.iter().position(|j| j.id == job_id);

        if let Some(index) = job_index {
            let mut job = active.remove(index);
            job.status = ExportStatus::Failed;

            self.completed_jobs.lock().unwrap().push(job);
            return true;
        }

        false
    }

    /// Transitions a pending export job to processing status.
    ///
    /// Called when the encoding pipeline begins actual frame processing.
    ///
    /// # Arguments
    ///
    /// * `job_id` - The `Uuid` of the export job to start processing.
    ///
    /// # Returns
    ///
    /// `true` if the job was found and transitioned, `false` otherwise.
    pub fn start_processing(&self, job_id: Uuid) -> bool {
        let mut active = self.active_jobs.lock().unwrap();
        if let Some(job) = active.iter_mut().find(|j| j.id == job_id) {
            job.status = ExportStatus::Processing;
            return true;
        }
        false
    }

    /// Returns a cloned copy of all active export jobs.
    ///
    /// Useful for the frontend to display a list of in-progress exports.
    pub fn get_active_jobs(&self) -> Vec<ExportJob> {
        self.active_jobs.lock().unwrap().clone()
    }

    /// Returns a cloned copy of all completed export jobs.
    ///
    /// Useful for the frontend to display export history.
    pub fn get_completed_jobs(&self) -> Vec<ExportJob> {
        self.completed_jobs.lock().unwrap().clone()
    }

    /// Returns the well-known export formats supported by FlowCut.
    ///
    /// This provides a static list of format definitions that the frontend
    /// can use to populate the export configuration UI. The actual availability
    /// of codecs within each format depends on the engine's codec list.
    pub fn get_export_formats() -> Vec<ExportFormat> {
        vec![
            ExportFormat {
                name: "MP4 (MPEG-4 Part 14)".to_string(),
                extension: "mp4".to_string(),
                description: "Universal format supported by most devices, browsers, and platforms. Ideal for web distribution and social media.".to_string(),
                codecs: vec!["h264".to_string(), "hevc".to_string(), "av1".to_string()],
                supported_resolutions: vec![
                    (3840, 2160), // 4K UHD
                    (2560, 1440), // QHD
                    (1920, 1080), // Full HD
                    (1280, 720),  // HD
                    (854, 480),   // SD
                ],
            },
            ExportFormat {
                name: "MKV (Matroska)".to_string(),
                extension: "mkv".to_string(),
                description: "Flexible container supporting virtually any codec. Ideal for archival and high-quality preservation with minimal compression loss.".to_string(),
                codecs: vec!["h264".to_string(), "hevc".to_string(), "vp9".to_string(), "av1".to_string(), "flac".to_string()],
                supported_resolutions: vec![
                    (3840, 2160),
                    (2560, 1440),
                    (1920, 1080),
                    (1280, 720),
                ],
            },
            ExportFormat {
                name: "MOV (QuickTime)".to_string(),
                extension: "mov".to_string(),
                description: "Apple's native format, widely used in professional video production and post-production workflows. Ideal for macOS/iOS targets.".to_string(),
                codecs: vec!["h264".to_string(), "hevc".to_string(), "prores".to_string()],
                supported_resolutions: vec![
                    (3840, 2160),
                    (1920, 1080),
                    (1280, 720),
                ],
            },
            ExportFormat {
                name: "WebM".to_string(),
                extension: "webm".to_string(),
                description: "Open web format optimized for HTML5 video streaming. Uses VP9/AV1 video and Opus audio. Ideal for web embedding.".to_string(),
                codecs: vec!["vp9".to_string(), "av1".to_string()],
                supported_resolutions: vec![
                    (1920, 1080),
                    (1280, 720),
                    (854, 480),
                    (640, 360),
                ],
            },
            ExportFormat {
                name: "GIF (Animated)".to_string(),
                extension: "gif".to_string(),
                description: "Animated image format with no audio support. Limited color palette (256 colors). Ideal for short looping clips and social media previews.".to_string(),
                codecs: vec!["gif".to_string()],
                supported_resolutions: vec![
                    (640, 480),
                    (480, 360),
                    (320, 240),
                ],
            },
            ExportFormat {
                name: "AVI (Audio Video Interleave)".to_string(),
                extension: "avi".to_string(),
                description: "Legacy Microsoft format with broad compatibility on Windows. Limited modern codec support. Suitable for older software integrations.".to_string(),
                codecs: vec!["h264".to_string(), "mpeg4".to_string()],
                supported_resolutions: vec![
                    (1920, 1080),
                    (1280, 720),
                ],
            },
            ExportFormat {
                name: "TS (MPEG-TS)".to_string(),
                extension: "ts".to_string(),
                description: "Transport stream format used for broadcast and streaming. Robust error recovery. Ideal for IPTV, satellite, and live streaming workflows.".to_string(),
                codecs: vec!["h264".to_string(), "hevc".to_string()],
                supported_resolutions: vec![
                    (1920, 1080),
                    (1280, 720),
                ],
            },
        ]
    }
}

impl Default for ExportState {
    /// Provides a default `ExportState` equivalent to `ExportState::new()`.
    fn default() -> Self {
        Self::new()
    }
}
