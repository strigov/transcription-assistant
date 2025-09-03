# Technical Specification: Transcription Assistant Application (Tauri-based)

## 1. Project Overview

### 1.1 Application Name
**Transcription Assistant** (Internal codename: TranscribeHelper)

### 1.2 Purpose
A lightweight, cross-platform desktop application designed to facilitate the transcription workflow by splitting audio/video files into manageable chunks and merging transcribed text files with proper timestamp synchronization.

### 1.3 Target Platforms
- Windows 10/11 (64-bit)
- macOS 11.0+ (Big Sur and later)

### 1.4 Technology Stack
- **Framework**: Tauri 1.5+ (Rust-based, uses system WebView)
- **Backend**: Rust (for core logic and system operations)
- **Frontend**: HTML5, CSS3, TypeScript
- **Build Tool**: Vite
- **UI Framework**: Vanilla JS with modern CSS (or optionally React/Vue/Svelte)
- **Media Processing**: FFmpeg (downloadable on demand)
- **Package Manager**: Cargo (Rust) + npm/pnpm (Frontend)
- **Target Application Size**: < 15 MB (without FFmpeg)

## 2. Architecture

### 2.1 Application Structure
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

### 2.2 Core Dependencies

#### Rust Dependencies
```toml
[dependencies]
tauri = { version = "1.5", features = ["shell-api", "fs-all", "dialog-all", "path-all"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
regex = "1.10"
chrono = "0.4"
reqwest = { version = "0.11", features = ["stream"] }
zip = "0.6"  # For FFmpeg extraction
```

#### Frontend Dependencies
```json
{
  "devDependencies": {
    "@tauri-apps/api": "^1.5.0",
    "typescript": "^5.0.0",
    "vite": "^4.4.0"
  }
}
```

## 3. Functional Requirements

### 3.1 Feature 1: Audio Extraction and Splitting

#### 3.1.1 Input Support
- Accept video files: MP4, AVI, MOV, MKV, WebM, FLV, WMV
- Accept audio files: MP3, WAV, AAC, FLAC, OGG, M4A, WMA, OPUS
- Maximum file size: 10 GB
- Drag-and-drop support via Tauri's drag-drop API
- File browser dialog using Tauri's dialog API

#### 3.1.2 Audio Processing
- Extract audio track from video files
- Output format: MP3 (configurable bitrate)
  - Options: 128kbps, 192kbps, 256kbps, 320kbps
- Split algorithm:
  - Maximum segment duration: 30 minutes (configurable: 10-60 min)
  - Smart splitting: detect silence gaps (>0.5s) near split points
  - Fallback: hard cut if no silence detected within ±10 seconds
- Progress reporting: real-time progress via Tauri events

#### 3.1.3 Output Configuration
- Naming pattern: `[original_filename]_part[N].mp3`
- Sequential numbering with zero-padding option (part001, part002)
- Custom output directory selection
- Preserve metadata where possible

#### 3.1.4 FFmpeg Management
- **On-demand download system**:
  - Initial app size: ~12-15 MB
  - FFmpeg download size: ~40 MB (minimal build) or ~80 MB (full build)
  - Download source: Official FFmpeg builds or custom CDN
- **Installation options**:
  1. Auto-download on first use
  2. Manual download via settings
  3. Use system FFmpeg if available
- **Storage location**:
  - Windows: `%APPDATA%/TranscriptionAssistant/ffmpeg/`
  - macOS: `~/Library/Application Support/TranscriptionAssistant/ffmpeg/`
- **Version management**:
  - Minimum required: FFmpeg 4.4+
  - Auto-update check (optional)

### 3.2 Feature 2: Transcription Merging

#### 3.2.1 Input Requirements
- Supported formats: TXT, SRT, MD
- Encoding support: UTF-8, UTF-16, ASCII
- Maximum files: 999 parts
- Total size limit: 500 MB combined

#### 3.2.2 Sequence Detection Algorithm
```rust
// Pattern matching priorities:
1. Numeric suffix: file_1.txt, file_2.txt
2. Part notation: file_part1.txt, file_part2.txt
3. Simple numbers: 1.txt, 2.txt, 10.txt
4. Zero-padded: file_001.txt, file_002.txt
5. Custom patterns via regex configuration
```

#### 3.2.3 Timestamp Processing Engine
- **Supported formats**:
  ```
  SRT:     00:00:00,000 --> 00:00:05,500
  WebVTT:  00:00:00.000 --> 00:00:05.500
  Simple:  [00:00:00] or [00:00]
  Markdown: **00:00:00** or __00:00:00__
  Custom:  User-defined regex patterns
  ```
- **Offset calculation**:
  - Automatic duration detection from previous parts
  - Manual offset override option
  - Precision: millisecond-level
- **Smart features**:
  - Overlap detection and warning
  - Gap detection and optional filling
  - Timestamp validation and repair

#### 3.2.4 Output Options
- Format selection: Same as input, SRT, WebVTT, Plain text
- Timestamp format customization
- Line ending style: LF, CRLF, CR
- Optional features:
  - Remove duplicate timestamps
  - Normalize timestamp format
  - Add speaker labels
  - Include part markers

## 4. User Interface Requirements

### 4.1 Design System

#### 4.1.1 Color Palette
```css
:root {
  /* Backgrounds */
  --bg-primary: #0f0f1e;
  --bg-secondary: #1a1a2e;
  --bg-tertiary: rgba(10, 10, 20, 0.8);
  
  /* Accent colors */
  --accent-primary: #1e3a8a;
  --accent-secondary: #3b82f6;
  --accent-hover: #2563eb;
  
  /* Text */
  --text-primary: #e0e0e0;
  --text-secondary: #a0a0a0;
  --text-muted: #606060;
  
  /* Status colors */
  --success: #10b981;
  --warning: #f59e0b;
  --error: #ef4444;
  --info: #3b82f6;
}
```

#### 4.1.2 Typography
- Font family: System font stack
- Headings: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI"
- Body: Same as headings
- Monospace: "Cascadia Code", "Fira Code", Consolas

### 4.2 Layout Components

#### 4.2.1 Main Window
- Minimum size: 800x600px
- Default size: 1024x768px
- Resizable with remembered dimensions
- Sidebar navigation (collapsible)
- Main content area with tabs

#### 4.2.2 Audio Processing View
```
┌─────────────────────────────────────┐
│ Drop Zone (Animated gradient border)│
│ "Drop video/audio files here"       │
│ [Browse Files] button               │
└─────────────────────────────────────┘

Settings Panel:
- Output Directory: [____] [Browse]
- Audio Quality: [Dropdown: 128/192/256/320]
- Max Duration: [30] minutes
- Smart Split: [✓] Detect silence

[Process Files] (Primary button)

Progress Section:
- Overall: [████████░░] 80%
- Current File: processing_part_3.mp3
- Time Remaining: ~2 minutes

Output Files:
┌─────────────────────────────────────┐
│ ✓ video_part1.mp3  (30:00)  2.1 MB │
│ ✓ video_part2.mp3  (30:00)  2.1 MB │
│ ⟳ video_part3.mp3  (15:30)  ...    │
└─────────────────────────────────────┘
```

#### 4.2.3 Merge Transcriptions View
```
┌─────────────────────────────────────┐
│ Drop Zone for Text Files            │
│ "Drop .txt/.srt/.md files here"     │
│ [Browse Files] button               │
└─────────────────────────────────────┘

File Order (Drag to reorder):
1. ≡ transcript_part1.srt
2. ≡ transcript_part2.srt
3. ≡ transcript_part3.srt
[Auto-detect order] [Clear all]

Settings:
- Output Format: [SRT ▼]
- Timestamp Style: [00:00:00,000 --> ▼]
- Add part markers: [✓]

[Merge Files] (Primary button)

Preview (First 500 chars):
┌─────────────────────────────────────┐
│ 1                                   │
│ 00:00:00,000 --> 00:00:05,500      │
│ This is the beginning of the...     │
└─────────────────────────────────────┘
```

### 4.3 Interactions & Animations
- Smooth transitions (200-300ms)
- Hover effects on interactive elements
- Loading spinners for async operations
- Progress animations
- Success/error toast notifications
- Subtle parallax on drop zones

## 5. FFmpeg Integration Strategy

### 5.1 Download Manager
```rust
pub struct FFmpegManager {
    // Check if FFmpeg is available
    pub fn is_installed(&self) -> bool;
    
    // Download FFmpeg (with progress callback)
    pub async fn download_ffmpeg(&self, 
        progress_callback: impl Fn(u64, u64)) -> Result<()>;
    
    // Verify FFmpeg installation
    pub fn verify_installation(&self) -> Result<Version>;
    
    // Get FFmpeg path
    pub fn get_ffmpeg_path(&self) -> PathBuf;
}
```

### 5.2 Download Flow
1. **First Launch Check**:
   - Check for system FFmpeg
   - Check for bundled FFmpeg
   - If not found, show download prompt

2. **Download Dialog**:
   ```
   FFmpeg Required
   
   This application requires FFmpeg to process media files.
   
   ○ Download minimal version (40 MB)
      - Audio processing only
      - Faster download
   
   ● Download full version (80 MB)
      - All codecs supported
      - Recommended
   
   ○ Use system FFmpeg
      - Select existing installation
   
   [Download] [Select Path] [Skip]
   ```

3. **Download Process**:
   - Show progress bar
   - Allow pause/resume
   - Verify checksum after download
   - Extract and set permissions

### 5.3 Platform-Specific Handling

#### Windows
- Download: ffmpeg-essential-win64.zip
- Extract to: `%LOCALAPPDATA%/TranscriptionAssistant/`
- Add to PATH temporarily during app runtime

#### macOS
- Download: ffmpeg-essential-macos.zip
- Extract to: `~/Library/Application Support/TranscriptionAssistant/`
- Handle Gatekeeper/notarization issues
- Set executable permissions: `chmod +x ffmpeg`

## 6. Performance Requirements

### 6.1 Benchmarks
- Application startup: < 2 seconds
- File processing speed: ≥ 10x realtime for audio
- Memory usage: < 200 MB idle, < 500 MB processing
- UI responsiveness: 60 FPS, < 16ms frame time

### 6.2 Optimization Strategies
- Lazy loading of UI components
- Web Workers for heavy computations
- Rust-based processing for performance-critical paths
- Streaming processing for large files
- Memory-mapped file I/O where applicable

## 7. Error Handling & Recovery

### 7.1 Error Categories
1. **FFmpeg Errors**:
   - Missing installation → Prompt download
   - Corrupted binary → Re-download option
   - Unsupported format → Clear message with alternatives

2. **File Processing Errors**:
   - Invalid input → Validation before processing
   - Write permissions → Request elevation or change directory
   - Disk space → Check before operation

3. **Network Errors** (during FFmpeg download):
   - Connection timeout → Retry with exponential backoff
   - Incomplete download → Resume capability
   - Checksum mismatch → Re-download

### 7.2 Recovery Mechanisms
- Auto-save progress for long operations
- Resume capability for interrupted processing
- Rollback on critical errors
- Detailed error logs in `~/.transcription-assistant/logs/`

## 8. Security & Privacy

### 8.1 Security Measures
- Code signing for distribution
- Checksum verification for FFmpeg downloads
- Sandboxed file system access
- No network access except for FFmpeg download
- No telemetry or data collection

### 8.2 Privacy Considerations
- All processing done locally
- No cloud services integration
- Temporary files cleaned automatically
- User consent for any external downloads

## 9. Packaging & Distribution

### 9.1 Build Configuration
```json
// tauri.conf.json excerpt
{
  "tauri": {
    "bundle": {
      "identifier": "com.transcription.assistant",
      "icon": ["icons/icon.ico", "icons/icon.icns"],
      "resources": [],
      "copyright": "© 2025 Transcription Assistant",
      "category": "Productivity",
      "shortDescription": "Split and merge transcription files",
      "longDescription": "Professional tool for audio splitting and transcription merging"
    }
  }
}
```

### 9.2 Distribution Packages

#### Windows
- **Installer**: NSIS-based (.exe) ~12 MB
- **Portable**: ZIP archive ~11 MB
- **Auto-updater**: MSI patches

#### macOS
- **DMG**: Drag-to-install ~13 MB

### 9.3 Update System
- Built-in update checker (optional)
- Differential updates when possible
- Manual update option always available
- Update changelog display

## 10. Testing Strategy

### 10.1 Unit Tests (Rust)
```rust
#[cfg(test)]
mod tests {
    // Test timestamp parsing
    // Test file sequence detection
    // Test offset calculations
    // Test FFmpeg command generation
}
```

### 10.2 Integration Tests
- FFmpeg download and installation
- Large file processing (>2GB)
- Various codec compatibility
- Cross-platform path handling

### 10.3 E2E Tests
- Complete workflow testing
- UI interaction testing with WebDriver
- Performance benchmarking
- Memory leak detection

### 10.4 Test Matrix
| Platform | Versions | Architectures |
|----------|----------|---------------|
| Windows | 10, 11 | x64, ARM64 |
| macOS | 11, 12, 13, 14 | Intel, Apple Silicon |


---

**Document Version**: 2.0 (Tauri-based)  
**Last Updated**: September 2025  
**Status**: Ready for Development  
**Estimated Package Size**: 12-15 MB (95-100 MB with FFmpeg)