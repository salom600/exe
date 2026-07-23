//! Video processing engine state module for FlowCut.
//!
//! This module defines the core engine state that manages FFmpeg availability,
//! GPU acceleration capabilities, supported media formats, and codec information.
//! The engine state is managed as a Tauri application state and is shared across
//! all Tauri commands via `std::sync::Mutex` for thread-safe access.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Information about a specific codec available on the system.
///
/// `CodecInfo` stores metadata about an individual codec, including whether
/// it supports encoding and/or decoding operations. This information is
/// gathered during engine initialization by probing FFmpeg's registered codecs.
///
/// # Examples
///
/// ```
/// use flowcut_lib::engine::CodecInfo;
///
/// let codec = CodecInfo {
///     name: "h264".to_string(),
///     description: "H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10".to_string(),
///     encode_support: true,
///     decode_support: true,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecInfo {
    /// The canonical name of the codec (e.g., "h264", "hevc", "aac").
    pub name: String,

    /// A human-readable description of the codec, typically sourced from
    /// FFmpeg's codec long_name field.
    pub description: String,

    /// Whether this codec can be used for encoding (writing) media streams.
    /// Some codecs may only be available for decoding if no encoder is
    /// installed on the system.
    pub encode_support: bool,

    /// Whether this codec can be used for decoding (reading) media streams.
    /// Most installed codecs support decoding, but proprietary or restricted
    /// codecs may not.
    pub decode_support: bool,
}

/// A snapshot of the engine's current operational status.
///
/// `EngineStatus` provides a read-only summary of the engine state, suitable
/// for returning to the frontend via Tauri commands. It does not contain
/// the full codec list or format list (which can be queried separately) but
/// instead provides summary counts and key boolean flags.
///
/// # Fields
///
/// - `is_initialized`: Whether the engine has completed its initialization
///   sequence, including FFmpeg probing and GPU detection.
/// - `ffmpeg_available`: Whether a usable FFmpeg binary was found on the system.
/// - `gpu_available`: Whether hardware-accelerated encoding/decoding is available.
/// - `supported_formats_count`: The number of media file formats the engine can handle.
/// - `active_jobs`: The number of currently running background processing jobs
///   (e.g., exports, thumbnail generation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatus {
    /// Whether the engine has been fully initialized and is ready for use.
    pub is_initialized: bool,

    /// Whether FFmpeg is available on the system and functional.
    pub ffmpeg_available: bool,

    /// Whether GPU-accelerated processing (encode/decode) is available.
    pub gpu_available: bool,

    /// The total count of media file formats the engine can read or write.
    pub supported_formats_count: usize,

    /// The number of currently active background processing jobs.
    pub active_jobs: usize,
}

/// System hardware and software information relevant to video editing.
///
/// `SystemInfo` is populated during engine initialization by querying the
/// operating system, CPU, memory, GPU driver, and FFmpeg version. This
/// information helps the frontend adapt its UI and processing strategies
/// based on the user's hardware capabilities.
///
/// # Examples
///
/// Typical values on a modern Linux workstation:
///
/// ```text
/// os_name:       "Linux (Ubuntu 22.04)"
/// cpu_cores:     16
/// total_memory_mb: 32768
/// gpu_name:      "NVIDIA GeForce RTX 4080"
/// ffmpeg_version: "6.0"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// The name and version of the operating system (e.g., "Windows 11",
    /// "macOS 14.2", "Linux (Ubuntu 22.04)").
    pub os_name: String,

    /// The number of logical CPU cores available for parallel processing.
    /// Used to determine optimal thread pool sizes for encoding and decoding.
    pub cpu_cores: usize,

    /// Total system RAM in megabytes. Used to estimate available memory for
    /// frame buffers, preview rendering, and export caching.
    pub total_memory_mb: u64,

    /// The name of the primary GPU device. If no GPU is detected or GPU
    /// acceleration is unavailable, this will be set to "N/A".
    pub gpu_name: String,

    /// The detected FFmpeg version string (e.g., "6.0", "5.1.2").
    /// If FFmpeg is not available, this will be set to "N/A".
    pub ffmpeg_version: String,
}

/// The central state manager for the FlowCut video processing engine.
///
/// `EngineState` is the top-level state object that is registered as a Tauri
/// managed state via `app.manage(EngineState::new())`. It tracks whether the
/// engine has been initialized, whether FFmpeg and GPU acceleration are
/// available, and maintains lists of supported formats and codecs.
///
/// # Thread Safety
///
/// All mutable fields are wrapped in `std::sync::Mutex` to ensure safe
/// concurrent access from multiple Tauri command handlers. Since Tauri
/// commands run on a multi-threaded runtime, the Mutex prevents data races
/// when multiple commands read or modify engine state simultaneously.
///
/// # Initialization Flow
///
/// 1. `EngineState::new()` creates a default (uninitialized) state.
/// 2. The `initialize_engine` command probes the system for FFmpeg, GPU, and
///    codec availability.
/// 3. Upon successful probing, `is_initialized` is set to `true` and the
///    format/codec lists are populated.
///
/// # Examples
///
/// ```rust
/// use flowcut_lib::engine::EngineState;
///
/// let state = EngineState::new();
/// // Initially uninitialized
/// assert!(!state.is_initialized.lock().unwrap());
/// ```
pub struct EngineState {
    /// Whether the engine has completed initialization.
    ///
    /// This flag is `false` when the application first starts and becomes
    /// `true` after the `initialize_engine` command successfully probes
    /// the system and populates all metadata fields.
    pub is_initialized: Mutex<bool>,

    /// Whether a usable FFmpeg binary is available on the system.
    ///
    /// Set during initialization by attempting to locate and execute the
    /// FFmpeg binary. If FFmpeg cannot be found or fails to respond to
    /// a version query, this remains `false` and most engine features
    /// will be unavailable.
    pub ffmpeg_available: Mutex<bool>,

    /// Whether GPU-accelerated video processing is available.
    ///
    /// Determined during initialization by querying FFmpeg for hardware
    /// acceleration support (e.g., NVENC, VA-API, VideoToolbox). If no
    /// supported GPU encoder/decoder is found, this remains `false`.
    pub gpu_available: Mutex<bool>,

    /// A list of media file formats the engine can read or write.
    ///
    /// Populated during initialization from FFmpeg's registered format
    /// demuxers and muxers. Common entries include "mp4", "mkv", "mov",
    /// "avi", "webm", "flv", "ts", and "gif".
    pub supported_formats: Mutex<Vec<String>>,

    /// A list of detailed codec information records.
    ///
    /// Each entry describes a codec's name, description, and whether it
    /// supports encoding and decoding. Populated during initialization
    /// by enumerating FFmpeg's registered audio and video codecs.
    pub codec_list: Mutex<Vec<CodecInfo>>,
}

impl EngineState {
    /// Creates a new `EngineState` with default (uninitialized) values.
    ///
    /// All boolean fields start as `false` and all lists start empty.
    /// The state must be explicitly initialized via the `initialize_engine`
    /// Tauri command before it can be used for video processing operations.
    ///
    /// # Returns
    ///
    /// A fresh `EngineState` instance ready for Tauri state management.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use flowcut_lib::engine::EngineState;
    ///
    /// let state = EngineState::new();
    /// assert!(!state.is_initialized.lock().unwrap());
    /// assert!(!state.ffmpeg_available.lock().unwrap());
    /// assert!(!state.gpu_available.lock().unwrap());
    /// assert!(state.supported_formats.lock().unwrap().is_empty());
    /// assert!(state.codec_list.lock().unwrap().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            is_initialized: Mutex::new(false),
            ffmpeg_available: Mutex::new(false),
            gpu_available: Mutex::new(false),
            supported_formats: Mutex::new(Vec::new()),
            codec_list: Mutex::new(Vec::new()),
        }
    }

    /// Retrieves a snapshot of the engine's current status.
    ///
    /// This method locks all relevant Mutex fields and constructs an
    /// `EngineStatus` struct that can be serialized and returned to the
    /// frontend. It provides a lightweight summary without the full codec
    /// or format lists.
    ///
    /// # Returns
    ///
    /// An `EngineStatus` reflecting the engine's current state.
    ///
    /// # Panics
    ///
    /// This method will panic if any Mutex is poisoned (i.e., a previous
    /// holder panicked while holding the lock). In practice, this should
    /// not occur in a well-behaved application.
    pub fn get_status(&self) -> EngineStatus {
        let is_initialized = *self.is_initialized.lock().unwrap();
        let ffmpeg_available = *self.ffmpeg_available.lock().unwrap();
        let gpu_available = *self.gpu_available.lock().unwrap();
        let supported_formats_count = self.supported_formats.lock().unwrap().len();

        EngineStatus {
            is_initialized,
            ffmpeg_available,
            gpu_available,
            supported_formats_count,
            active_jobs: 0, // TODO: integrate with job tracking system
        }
    }

    /// Retrieves the full list of supported media formats.
    ///
    /// Returns a cloned copy of the format list to avoid holding the
    /// Mutex lock across a serialization boundary.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` of format names (e.g., "mp4", "mkv", "mov").
    pub fn get_supported_formats(&self) -> Vec<String> {
        self.supported_formats.lock().unwrap().clone()
    }

    /// Retrieves the full list of available codec information records.
    ///
    /// Returns a cloned copy of the codec list to avoid holding the
    /// Mutex lock across a serialization boundary.
    ///
    /// # Returns
    ///
    /// A `Vec<CodecInfo>` describing all detected codecs.
    pub fn get_codec_list(&self) -> Vec<CodecInfo> {
        self.codec_list.lock().unwrap().clone()
    }

    /// Marks the engine as initialized and populates all metadata fields.
    ///
    /// This method is called by the `initialize_engine` Tauri command after
    /// successfully probing the system. It atomically updates all fields
    /// under their respective Mutex locks.
    ///
    /// # Arguments
    ///
    /// * `ffmpeg_available` - Whether FFmpeg was found and is functional.
    /// * `gpu_available` - Whether GPU acceleration was detected.
    /// * `supported_formats` - The list of media formats FFmpeg can handle.
    /// * `codec_list` - The list of codec information records from FFmpeg.
    pub fn initialize(
        &self,
        ffmpeg_available: bool,
        gpu_available: bool,
        supported_formats: Vec<String>,
        codec_list: Vec<CodecInfo>,
    ) {
        *self.is_initialized.lock().unwrap() = true;
        *self.ffmpeg_available.lock().unwrap() = ffmpeg_available;
        *self.gpu_available.lock().unwrap() = gpu_available;
        *self.supported_formats.lock().unwrap() = supported_formats;
        *self.codec_list.lock().unwrap() = codec_list;
    }
}

impl Default for EngineState {
    /// Provides a default `EngineState` equivalent to `EngineState::new()`.
    ///
    /// This implementation allows `EngineState` to be used with any API
    /// that requires a `Default` trait bound, such as certain Tauri state
    /// management patterns.
    fn default() -> Self {
        Self::new()
    }
}
