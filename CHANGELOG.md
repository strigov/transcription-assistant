# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Drag & drop file support (in development)
- Batch processing for multiple files
- Built-in Whisper integration for transcription

### Changed
- Improved error handling and user feedback

### Fixed
- File path handling on different platforms

## [0.1.0] - 2024-01-XX - Initial Release

### Added
- **Audio Processing Features**
  - Smart audio/video file splitting with configurable duration (1-60 minutes)
  - Support for 15+ media formats (MP4, AVI, MOV, MKV, WebM, FLV, WMV, MP3, WAV, AAC, FLAC, OGG, M4A, WMA, OPUS)
  - Silence detection for intelligent splitting at natural breaks
  - Time-based splitting as fallback option
  - High-quality MP3 output with 128k bitrate

- **FFmpeg Integration**
  - Automatic FFmpeg download and management
  - Cross-platform support (Windows, macOS, Linux)
  - System FFmpeg detection and usage
  - Secure checksum verification

- **Transcription Management**
  - Merge multiple transcription files (TXT, SRT, MD formats)
  - Intelligent file sequence detection
  - Timestamp synchronization with offset calculation
  - Export to multiple formats

- **User Interface**
  - Beautiful modern UI with gradient design
  - Real-time progress tracking with detailed status
  - File information display (name, duration, size)
  - Processing results with segment details
  - One-click folder access and path copying
  - Visual feedback for all operations

- **Core Functionality**
  - Local-only processing for privacy
  - Configurable processing options
  - Error handling with user-friendly messages
  - Output files saved alongside source files
  - Cross-platform desktop application

### Technical Details
- **Frontend**: HTML5, CSS3, TypeScript with Vite build system
- **Backend**: Rust with Tauri framework and tokio async runtime
- **Media Processing**: FFmpeg with libmp3lame encoder
- **File Management**: Smart directory creation and path handling
- **Architecture**: Event-driven with progress callbacks

### Known Issues
- Drag & drop functionality temporarily disabled (use Select File buttons)
- Long filenames may cause display overflow
- Some exotic formats require manual FFmpeg installation

### Security
- All processing performed locally
- No telemetry or data collection
- Sandboxed file system access
- Secure FFmpeg binary verification