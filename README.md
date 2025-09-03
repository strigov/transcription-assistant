# 🎙️ Transcription Assistant

A powerful desktop application built with Tauri that helps you split audio/video files into manageable chunks and merge transcribed text files with proper timestamp synchronization.

![Transcription Assistant](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Tauri](https://img.shields.io/badge/Tauri-1.5+-orange)

## ✨ Features

### 🎵 Audio Processing
- **Smart Audio Splitting**: Automatically split long audio/video files into chunks
- **Multiple Formats**: Supports MP4, AVI, MOV, MKV, WebM, FLV, WMV, MP3, WAV, AAC, FLAC, OGG, M4A, WMA, OPUS
- **Silence Detection**: Intelligent splitting based on silence detection for natural breaks
- **Time-based Splitting**: Configurable maximum segment duration (default: 30 minutes)
- **High-Quality Output**: MP3 segments with 128k bitrate for optimal size/quality balance

### 📝 Transcription Management
- **File Merging**: Combine multiple transcription files with proper sequencing
- **Multiple Formats**: Support for TXT, SRT, and Markdown files
- **Timestamp Synchronization**: Automatic offset calculation for seamless merging
- **Export Options**: Export merged transcriptions in various formats

### 🛠️ User Experience
- **Beautiful Interface**: Modern, intuitive UI with progress tracking
- **Drag & Drop**: Easy file management (coming soon)
- **Local Processing**: All operations performed locally for privacy
- **Cross-Platform**: Windows, macOS, and Linux support
- **One-Click Access**: Open output folders directly from the app

## 🚀 Getting Started

### Prerequisites
- Node.js 16+ and npm/pnpm
- Rust 1.70+
- FFmpeg (automatically downloaded by the app)

### Installation

#### Option 1: Download Release (Recommended)
1. Go to [Releases](https://github.com/your-username/transcription-assistant/releases)
2. Download the installer for your platform:
   - **Windows**: `transcription-assistant_x.x.x_x64-setup.exe`
   - **macOS**: `transcription-assistant_x.x.x_x64.dmg`
   - **Linux**: `transcription-assistant_x.x.x_amd64.deb` or `.AppImage`
3. Install and run

#### Option 2: Build from Source
```bash
# Clone the repository
git clone https://github.com/your-username/transcription-assistant.git
cd transcription-assistant

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

## 📖 How to Use

### 1. **Select Audio/Video File**
- Click "Select File" or use drag & drop
- Supported formats: Most common audio/video formats
- File info (duration, size) will be displayed

### 2. **Configure Processing**
- Set maximum segment duration (1-60 minutes)
- Choose between silence detection or time-based splitting
- Click "🔄 Start Processing"

### 3. **Review Results**
- View created audio segments with details
- Use "📁 Open Folder" to access files
- Use "📋 Copy Path" for external tools

### 4. **Merge Transcriptions**
- Add transcribed text files (TXT, SRT, MD)
- Click "🔗 Merge Transcriptions"
- Files will be combined with proper sequencing

### 5. **Export**
- Choose output format (TXT, SRT, MD)
- Click "💾 Export" to save merged transcription
- Files are saved to Documents folder with timestamp

## 🏗️ Architecture

### Technology Stack
- **Frontend**: HTML5, CSS3, TypeScript with Vite
- **Backend**: Rust with Tauri framework
- **Media Processing**: FFmpeg integration
- **Async Operations**: Tokio runtime

### Project Structure
```
transcription-assistant/
├── src/                    # Frontend source code
│   ├── index.html         # Main HTML file
│   ├── main.ts           # TypeScript main logic
│   └── styles/           # CSS styles
├── src-tauri/            # Rust backend
│   ├── src/
│   │   ├── main.rs      # Entry point
│   │   ├── commands.rs  # Tauri commands
│   │   ├── audio.rs     # Audio processing
│   │   ├── merger.rs    # Text merging
│   │   └── ffmpeg.rs    # FFmpeg integration
│   ├── Cargo.toml       # Rust dependencies
│   └── tauri.conf.json  # Tauri configuration
├── package.json          # Frontend dependencies
└── vite.config.ts       # Vite configuration
```

## 🔧 Development

### Available Scripts
```bash
# Development
npm run tauri:dev        # Start dev server with hot reload
npm run dev             # Frontend only
npm run build           # Build frontend

# Production
npm run tauri:build     # Build app for production
npm run tauri:build -- --debug  # Debug build

# Testing
cd src-tauri && cargo test  # Run Rust tests
npm test                # Frontend tests (when implemented)
```

### Key Dependencies

**Rust (Backend)**
- `tauri`: Cross-platform app framework
- `tokio`: Async runtime
- `serde`: Serialization
- `ffmpeg`: Media processing
- `chrono`: Date/time handling

**Frontend**
- `@tauri-apps/api`: Tauri JavaScript bindings
- `typescript`: Type safety
- `vite`: Build tool

## 🤝 Contributing

We welcome contributions! Please feel free to submit a Pull Request.

### Development Guidelines
1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

### Reporting Issues
Please use the [Issue Tracker](https://github.com/your-username/transcription-assistant/issues) to report bugs or request features.

## 📋 Roadmap

- [ ] **v0.2.0**: Drag & drop file support
- [ ] **v0.3.0**: Batch processing multiple files
- [ ] **v0.4.0**: Built-in transcription with Whisper integration
- [ ] **v0.5.0**: Cloud storage integration
- [ ] **v1.0.0**: Plugin system for custom workflows

## 🐛 Known Issues

- Drag & drop functionality is temporarily disabled (use Select File buttons)
- Very long filenames may cause display issues
- Some exotic audio formats may require manual FFmpeg installation

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) - Fantastic cross-platform framework
- [FFmpeg](https://ffmpeg.org/) - Powerful multimedia processing
- [Rust Community](https://www.rust-lang.org/community) - Amazing ecosystem

## 📞 Support

- 📧 Email: your-email@example.com
- 🐛 Issues: [GitHub Issues](https://github.com/your-username/transcription-assistant/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/your-username/transcription-assistant/discussions)

---

**Made with ❤️ and Rust**

If you find this project helpful, please consider giving it a ⭐ on GitHub!