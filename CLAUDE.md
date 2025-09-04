# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Tauri-based desktop application for audio/video transcription workflow assistance. The app splits audio/video files into manageable chunks and merges transcribed text files with proper timestamp synchronization.

**Current Status**: Project has complete Tauri application structure with modern dark-themed frontend UI featuring working drag-and-drop file handling and Rust backend configured. Core modules (FFmpeg integration, audio processing, transcription merging) are pending implementation.

## Architecture

### Technology Stack
- **Framework**: Tauri 1.5+ (Rust backend + web frontend)
- **Backend**: Rust with tokio for async operations
- **Frontend**: HTML5, CSS3, TypeScript with Vite build tool
- **Media Processing**: FFmpeg (downloadable on demand)
- **Package Managers**: Cargo (Rust) + npm/pnpm (Frontend)

### Project Structure
```
transcription-assistant/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── audio.rs       # Audio processing logic
│   │   ├── merger.rs      # Text merging logic
│   │   ├── ffmpeg.rs      # FFmpeg integration
│   │   └── commands.rs    # Tauri commands
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                    # Frontend
│   ├── index.html
│   ├── main.ts
│   ├── styles/
│   ├── scripts/
│   └── components/
├── package.json
└── vite.config.ts
```

## Development Commands

### Initial Setup
```bash
# Create Tauri project
npm create tauri-app@latest
# or
cargo install create-tauri-app
cargo create-tauri-app

# Install frontend dependencies
npm install

# Install Rust dependencies (done automatically with cargo)
cd src-tauri && cargo build
```

### Development
```bash
# Install dependencies
npm install

# Start development server (launches Tauri app with hot reload)
npm run tauri:dev

# Build frontend only
npm run build

# Build for development with debug symbols
npm run tauri:build -- --debug

# Build for production
npm run tauri:build
```

### Testing
```bash
# Run Rust unit tests
cd src-tauri && cargo test

# Run frontend tests (when implemented)
npm test

# Integration tests
cargo test --features=integration-tests
```

## Core Dependencies

### Rust Dependencies (src-tauri/Cargo.toml)
```toml
[dependencies]
tauri = { version = "1.5", features = ["shell-api", "fs-all", "dialog-all", "path-all"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
regex = "1.10"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.11", features = ["stream"] }
zip = "0.6"
anyhow = "1.0"
thiserror = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
```

### Frontend Dependencies
```json
{
  "dependencies": {
    "@tauri-apps/api": "^1.5.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^1.5.0",
    "typescript": "^5.0.0",
    "vite": "^4.4.0"
  }
}
```

## Current Implementation Architecture

### Frontend Structure
- **Main Application**: `src/main.ts` - TypeScript class-based architecture with `TranscriptionAssistant` class managing UI state and Tauri API interactions
- **UI Components**: HTML5-based interface with modern dark theme, sections for file input, processing options, progress tracking, transcription merging, and output export
- **Event System**: Uses Tauri's event system for real-time progress updates from Rust backend
- **File Handling**: Full drag-and-drop support via Tauri's native file drop API (`tauri://file-drop` events) and traditional file dialogs

### Backend Structure (Rust)
- **Entry Point**: `src-tauri/src/main.rs` (not yet implemented)
- **Tauri Commands**: Backend exposes commands like `get_file_info`, `start_audio_processing`, `merge_transcriptions`
- **Event Emission**: Progress updates sent to frontend via Tauri events (`processing-progress`, `processing-complete`)

### Key Tauri Commands (Frontend → Backend)
- `get_file_info(path)`: Returns media file metadata (name, duration, size)
- `start_audio_processing(filePath, maxDuration, useSilenceDetection)`: Initiates audio splitting
- `merge_transcriptions(files, outputFormat)`: Merges transcription files with timestamp sync
- `export_merged_transcription()`: Exports final merged transcription

### Drag-and-Drop Implementation
- **Tauri Configuration**: `fileDropEnabled: true` in window config enables native file drop support
- **Event Handler**: `tauri://file-drop` event listener processes dropped files with automatic type detection
- **File Type Recognition**: Automatic sorting by extension (media files vs transcription files)
- **Supported Extensions**:
  - Media: mp4, avi, mov, mkv, webm, flv, wmv, mp3, wav, aac, flac, ogg, m4a, wma, opus
  - Transcriptions: txt, srt, md
- **UI Feedback**: Visual feedback and automatic interface updates when files are dropped

### Supported File Formats
- **Media Input**: MP4, AVI, MOV, MKV, WebM, FLV, WMV, MP3, WAV, AAC, FLAC, OGG, M4A, WMA, OPUS
- **Transcription Input/Output**: TXT, SRT, MD

## Key Features to Implement

### 1. Audio Processing Module
- Support for video formats: MP4, AVI, MOV, MKV, WebM, FLV, WMV
- Support for audio formats: MP3, WAV, AAC, FLAC, OGG, M4A, WMA, OPUS
- Smart audio splitting with silence detection
- Maximum segment duration of 30 minutes (configurable)
- Progress reporting via Tauri events

### 2. FFmpeg Integration
- On-demand download system (40-80 MB download)
- Storage locations:
  - Windows: `%APPDATA%/TranscriptionAssistant/ffmpeg/`
  - macOS: `~/Library/Application Support/TranscriptionAssistant/ffmpeg/`
- System FFmpeg detection and usage

### 3. Transcription Merging
- Support for TXT, SRT, MD formats
- Intelligent file sequence detection
- Timestamp synchronization with offset calculation
- Multiple timestamp format support

## Platform-Specific Considerations

### Windows
- Use NSIS installer
- Handle Windows path conventions
- FFmpeg permissions and PATH management

### macOS  
- Handle Gatekeeper/notarization
- DMG distribution
- Set proper executable permissions for FFmpeg

## Performance Requirements
- Application startup: < 2 seconds
- Processing speed: ≥ 10x realtime for audio
- Memory usage: < 200 MB idle, < 500 MB processing
- UI responsiveness: 60 FPS

## Security & Privacy
- All processing done locally
- No telemetry or data collection
- Code signing for distribution
- Checksum verification for FFmpeg downloads
- Sandboxed file system access