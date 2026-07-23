//! Engine management commands for FlowCut.
//!
//! This module provides Tauri command handlers for initializing the
//! FFmpeg video processing engine and querying its runtime status
//! and system capabilities. The engine is the core component that
//! powers all media decoding, preview rendering, and export encoding
//! operations.
//!
//! # Initialization
//!
//! The engine must be initialized via [`initialize_engine`] before any
//! media processing commands can be used. Initialization probes the
//! system for available FFmpeg libraries, GPU acceleration support,
//! and codec availability.
//!
//! # State Dependencies
//!
//! Commands depend on [`EngineState`] for engine lifecycle management.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::engine::EngineState;

/// Runtime status of the video processing engine.
///
/// Provides a comprehensive snapshot of the engine's current state,
/// including initialization status, supported codecs, hardware
/// acceleration capabilities, and resource usage.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EngineStatus {
    /// Whether the engine has been successfully initialized.
    pub initialized: bool,
    /// Version of the FFmpeg libraries loaded (e.g. "6.1.2").
    pub ffmpeg_version: String,
    /// Version of the FFmpeg libavcodec library.
    pub libavcodec_version: String,
    /// Version of the FFmpeg libavformat library.
    pub libavformat_version: String,
    /// Version of the FFmpeg libavutil library.
    pub libavutil_version: String,
    /// Whether hardware-accelerated decoding is available.
    pub hw_accel_available: bool,
    /// Type of hardware acceleration detected (e.g. "cuda", "qsv",
    /// "vaapi", "videotoolbox", "d3d11va", or "none").
    pub hw_accel_type: String,
    /// Number of supported video decoder codecs.
    pub video_decoders_count: usize,
    /// Number of supported video encoder codecs.
    pub video_encoders_count: usize,
    /// Number of supported audio decoder codecs.
    pub audio_decoders_count: usize,
    /// Number of supported audio encoder codecs.
    pub audio_encoders_count: usize,
    /// Number of supported container formats (muxers).
    pub muxers_count: usize,
    /// Number of supported demuxer formats.
    pub demuxers_count: usize,
    /// Number of supported video filters.
    pub video_filters_count: usize,
    /// Number of supported audio filters.
    pub audio_filters_count: usize,
    /// Whether the engine is currently processing a task (rendering,
    /// encoding, or decoding).
    pub busy: bool,
    /// Current memory usage by the engine in bytes.
    pub memory_usage: u64,
    /// Peak memory usage since initialization in bytes.
    pub peak_memory_usage: u64,
}

/// System hardware and software capabilities.
///
/// Provides information about the host system that may affect
/// performance and feature availability, such as CPU cores,
/// available RAM, GPU model, and OS version. The frontend can
/// use this to display system info in a settings panel or to
/// make intelligent decisions about preview resolution and
/// export performance expectations.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemInfo {
    /// Operating system name (e.g. "macOS", "Windows", "Linux").
    pub os_name: String,
    /// Operating system version string (e.g. "14.2.1", "11", "6.5.0").
    pub os_version: String,
    /// CPU architecture (e.g. "x86_64", "aarch64", "arm64").
    pub cpu_arch: String,
    /// Number of logical CPU cores available for parallel processing.
    pub cpu_cores: u32,
    /// Total physical RAM in bytes.
    pub total_ram: u64,
    /// Available (free) RAM in bytes at the time of query.
    pub available_ram: u64,
    /// GPU model name, if a discrete GPU is present
    /// (e.g. "NVIDIA GeForce RTX 4090", "Apple M2 GPU").
    pub gpu_model: Option<String>,
    /// GPU driver version, if applicable.
    pub gpu_driver_version: Option<String>,
    /// Available GPU memory in bytes, if a discrete GPU is detected.
    pub gpu_memory: Option<u64>,
    /// Maximum supported video resolution based on GPU capabilities.
    pub max_gpu_resolution: Option<String>,
    /// Available disk space on the system drive in bytes.
    pub disk_space: u64,
    /// Screen resolution of the primary display
    /// (e.g. "3840x2160", "1920x1080").
    pub screen_resolution: String,
    /// Number of connected displays.
    pub display_count: u32,
}

/// Error type for engine command failures.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EngineError {
    /// Machine-readable error category.
    pub kind: String,
    /// Human-readable error description.
    pub message: String,
}

/// Initializes the video processing engine.
///
/// Probes the system for FFmpeg libraries, validates codec availability,
/// detects hardware acceleration capabilities, and prepares the engine
/// for media processing operations. This must be called before any
/// commands that depend on the engine (import, preview, export).
///
/// # Returns
///
/// `true` if the engine initialized successfully, or an [`EngineError`]
/// if:
/// - FFmpeg libraries cannot be found or loaded.
/// - The minimum required codecs are not available.
/// - Hardware acceleration initialization fails (this is non-fatal;
///   the engine falls back to software decoding, but the error is
///   reported for informational purposes).
///
/// # Re-initialization
///
/// Calling this on an already-initialized engine is safe — it will
/// re-probe the system and update the status, but will not interrupt
/// any active processing tasks.
#[tauri::command]
pub fn initialize_engine(engine_state: State<EngineState>) -> Result<bool, EngineError> {
    log::info!("Initializing the video processing engine");

    let mut engine = engine_state.data.lock().map_err(|e| EngineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    // If already initialized, re-probe but don't fail
    if engine.is_initialized() {
        log::info!("Engine already initialized — re-probing system capabilities");
        engine.reprobe().map_err(|e| EngineError {
            kind: "reprobe_failed".into(),
            message: format!("Failed to re-probe engine: {}", e),
        })?;
        return Ok(true);
    }

    // Perform initial engine setup: load FFmpeg libs, detect HW accel,
    // enumerate codecs
    engine.initialize().map_err(|e| EngineError {
        kind: "initialization_failed".into(),
        message: format!("Failed to initialize the video engine: {}", e),
    })?;

    log::info!("Video engine initialized successfully");
    Ok(true)
}

/// Retrieves the current status of the video processing engine.
///
/// Returns a comprehensive snapshot of the engine's runtime state,
/// including codec counts, hardware acceleration details, and
/// resource usage metrics. This is useful for the frontend to
/// display engine information in a settings or diagnostics panel.
///
/// # Returns
///
/// An [`EngineStatus`] struct with the current engine state, or an
/// [`EngineError`] if the engine state cannot be read.
///
/// # Note
///
/// This command can be called even if the engine is not initialized.
/// In that case, the returned [`EngineStatus`] will have
/// `initialized: false` and most fields will be empty or zero.
#[tauri::command]
pub fn get_engine_status(engine_state: State<EngineState>) -> Result<EngineStatus, EngineError> {
    log::info!("Retrieving engine status");

    let engine = engine_state.data.lock().map_err(|e| EngineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    let status = engine.get_status().map_err(|e| EngineError {
        kind: "status_failed".into(),
        message: format!("Failed to get engine status: {}", e),
    })?;

    Ok(EngineStatus {
        initialized: status.initialized,
        ffmpeg_version: status.ffmpeg_version,
        libavcodec_version: status.libavcodec_version,
        libavformat_version: status.libavformat_version,
        libavutil_version: status.libavutil_version,
        hw_accel_available: status.hw_accel_available,
        hw_accel_type: status.hw_accel_type,
        video_decoders_count: status.video_decoders_count,
        video_encoders_count: status.video_encoders_count,
        audio_decoders_count: status.audio_decoders_count,
        audio_encoders_count: status.audio_encoders_count,
        muxers_count: status.muxers_count,
        demuxers_count: status.demuxers_count,
        video_filters_count: status.video_filters_count,
        audio_filters_count: status.audio_filters_count,
        busy: status.busy,
        memory_usage: status.memory_usage,
        peak_memory_usage: status.peak_memory_usage,
    })
}

/// Retrieves information about the host system's hardware and software.
///
/// Probes the system for CPU, RAM, GPU, disk, and display information.
/// This is useful for:
/// - Displaying system specifications in the settings panel.
/// - Automatically selecting optimal preview resolutions.
/// - Estimating export performance based on available resources.
/// - Warning the user if their system may not meet minimum
///   requirements for certain operations.
///
/// # Returns
///
/// A [`SystemInfo`] struct with the host system's capabilities, or an
/// [`EngineError`] if the system probe fails.
///
/// # Performance
///
/// This command performs a lightweight system probe and should complete
/// in under 100ms. It does not depend on the engine being initialized.
#[tauri::command]
pub fn get_system_info(engine_state: State<EngineState>) -> Result<SystemInfo, EngineError> {
    log::info!("Retrieving system information");

    let engine = engine_state.data.lock().map_err(|e| EngineError {
        kind: "state_lock".into(),
        message: format!("Failed to acquire engine state lock: {}", e),
    })?;

    let info = engine.get_system_info().map_err(|e| EngineError {
        kind: "system_info_failed".into(),
        message: format!("Failed to retrieve system info: {}", e),
    })?;

    Ok(SystemInfo {
        os_name: info.os_name,
        os_version: info.os_version,
        cpu_arch: info.cpu_arch,
        cpu_cores: info.cpu_cores,
        total_ram: info.total_ram,
        available_ram: info.available_ram,
        gpu_model: info.gpu_model,
        gpu_driver_version: info.gpu_driver_version,
        gpu_memory: info.gpu_memory,
        max_gpu_resolution: info.max_gpu_resolution,
        disk_space: info.disk_space,
        screen_resolution: info.screen_resolution,
        display_count: info.display_count,
    })
}
