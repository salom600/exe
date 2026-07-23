# FlowCut — Professional Lightweight Video Editor

![FlowCut Logo](src/assets/icons/flowcut.svg)

**FlowCut** is a professional, lightweight, and fast video editing application built with modern technologies: **Rust**, **Tauri 2**, and **FFmpeg**. It provides a full-featured non-linear editing (NLE) experience with a clean, professional dark-themed interface.

## Features

### Core Editing
- 🎬 **Timeline-based NLE** — Multi-track video and audio editing with professional timeline interface
- ✂️ **Cut, Trim, Split** — Full clip manipulation with razor, slip, slide, and ripple edit tools
- 🔄 **Transitions** — Crossfade, dissolve, fade in/out, wipe transitions
- 🔀 **Multi-track Support** — Video, audio, and text tracks with independent controls
- ⏩ **Speed Ramping** — Adjust playback speed from 0.1x to 10x

### Effects & Filters
- 🎨 **Color Correction** — Brightness, contrast, saturation, hue adjustments
- 🔍 **Built-in Filters** — Comprehensive filter library with real-time preview
- 📊 **Keyframe Animation** — Animate filter parameters over time
- 🔌 **Extensible Plugin System** — Architecture designed for future effect plugins

### Media Management
- 📂 **Media Browser** — Import, browse, search, and filter media files
- 🖼️ **Thumbnail Preview** — Visual media thumbnails with file type badges
- 🎯 **Drag-to-Timeline** — Drag media items directly onto timeline tracks

### Preview & Playback
- 📺 **Real-time Preview** — Full, half, or quarter quality preview modes
- 🎮 **Transport Controls** — Play, pause, stop, frame-by-frame navigation
- 🕐 **Timecode Display** — Professional HH:MM:SS:FF timecode format
- 🔊 **Volume Control** — Audio level adjustment with mute toggle

### Export
- 📦 **Multi-format Export** — MP4 (H.264/H.265), MKV, MOV, WebM (VP9), AVI, GIF, TS
- ⚙️ **Quality Presets** — UltraFast to VerySlow encoding speed control
- 🎯 **Resolution Options** — SD (480p), HD (720p), Full HD (1080p), 4K UHD (2160p)
- 📊 **Export Progress** — Real-time progress tracking with FPS and time estimates

### Professional UI
- 🌙 **Dark Theme** — Catppuccin Mocha-based professional dark interface
- ⌨️ **Keyboard Shortcuts** — 28+ professional shortcuts for efficient editing
- 🔒 **Track Locking** — Lock and hide tracks to prevent accidental edits
- 📌 **Markers** — Add named markers with colors to timeline positions
- ↩️ **Undo/Redo** — Full undo history (up to 100 actions)

## Architecture

```
FlowCut/
├── src-tauri/           # Rust backend (Tauri application)
│   ├── src/
│   │   ├── commands/    # Tauri IPC command handlers (9 modules)
│   │   ├── engine/      # Video processing engine state
│   │   ├── project/     # Project & timeline data models
│   │   ├── export/      # Export management & job tracking
│   │   └── utils/       # Undo manager & utility types
│   ├── Cargo.toml       # Rust dependencies
│   └── tauri.conf.json  # Tauri application config
├── src/                 # Frontend (HTML/CSS/JS)
│   ├── index.html       # Main application shell
│   ├── styles/          # 7 CSS modules (Catppuccin Mocha theme)
│   ├── scripts/         # 11 JS modules (core, timeline, preview, media, UI)
│   └── assets/          # Icons and fonts
├── .github/workflows/   # CI/CD (build.yml + release.yml)
└── package.json         # Node.js configuration
```

### Technology Stack
| Component | Technology | Purpose |
|-----------|-----------|---------|
| Backend | Rust + Tauri 2 | Memory-safe, fast core engine |
| Video Processing | FFmpeg | Industry-standard codec support |
| Frontend | HTML5/CSS3/JS | Professional NLE interface |
| IPC | Tauri Commands | Bidirectional Rust↔UI communication |
| State Management | std::sync::Mutex | Thread-safe shared state |
| Build | GitHub Actions | Cross-platform CI/CD |

## Building from Source

### Prerequisites

**All platforms:**
- Rust 1.75+ (`rustup`)
- Node.js 20+
- npm

**Linux (Ubuntu/Debian):**
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libffmpeg-dev libavcodec-dev libavformat-dev libavutil-dev \
  libswscale-dev libswresample-dev libsoup-3.0-dev libssl-dev
```

**Windows:**
- Microsoft Visual Studio C++ Build Tools
- WebView2 (included in Windows 10+)

**macOS:**
- Xcode Command Line Tools (`xcode-select --install`)

### Build Steps

```bash
# Clone the repository
git clone https://github.com/salom600/exe.git
cd exe

# Install npm dependencies
npm install

# Build in development mode
npm run dev

# Build for production
npm run build
```

The build output will be in `src-tauri/target/release/bundle/` with platform-specific packages.

### Development Mode

```bash
npm run dev
```

This launches the application with hot-reload for the frontend and debug builds for the Rust backend.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New Project |
| `Ctrl+O` | Open Project |
| `Ctrl+S` | Save Project |
| `Ctrl+E` | Export Video |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |
| `Space` | Play/Pause |
| `S` | Split at Playhead |
| `Delete` | Delete Selected Clip |
| `V` | Selection Tool |
| `C` | Razor Tool |
| `Y` | Slip Tool |
| `U` | Slide Tool |
| `B` | Ripple Edit Tool |
| `+/-` | Zoom In/Out |
| `←/→` | Previous/Next Frame |
| `F11` | Fullscreen Preview |
| `Ctrl+A` | Select All |
| `Ctrl+X/C/V` | Cut/Copy/Paste |

## GitHub Actions CI/CD

This project includes comprehensive GitHub Actions workflows:

### `build.yml` — Continuous Integration
Triggers on push to main/master branches and pull requests:
1. **Rust Tests** — Format check, Clippy lint, build, and test
2. **Frontend Check** — Validate all HTML, CSS, and JS files exist
3. **Build Linux** — Build `.deb` and `.AppImage` packages
4. **Build Windows** — Build `.exe` and NSIS installer
5. **Build macOS** — Build `.app` bundle and `.dmg`

### `release.yml` — Release Publishing
Triggers on version tags (`v*`) or manual dispatch:
1. Creates a GitHub Release with changelog
2. Builds all platform packages
3. Uploads installers to the release

### GitHub Actions Free Tier Limits
- **6 hours** maximum per job execution
- **2,000 minutes** per month (free tier)
- This project uses aggressive Cargo caching to minimize build times
- Typical full build: ~15-20 minutes with caching

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## License

FlowCut is licensed under the **GNU General Public License v3.0** — see [LICENSE](LICENSE) for details.

## Project Status

FlowCut is actively developed. Current version: **1.0.0**

### Roadmap
- GPU-accelerated rendering via Vulkan/OpenGL
- AI-assisted scene detection
- Proxy editing workflow for large files
- LUT (Look-Up Table) support for color grading
- Custom effect plugin API
- Multi-language UI support
- Network collaboration features
