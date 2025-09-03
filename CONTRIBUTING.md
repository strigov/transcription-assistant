# Contributing to Transcription Assistant

Thank you for your interest in contributing to Transcription Assistant! This document provides guidelines and information for contributors.

## 🤝 How to Contribute

### Reporting Issues
- Use the [Issue Tracker](https://github.com/your-username/transcription-assistant/issues)
- Search existing issues before creating new ones
- Provide clear, detailed information including:
  - Operating system and version
  - Steps to reproduce
  - Expected vs actual behavior
  - Screenshots/logs if applicable

### Feature Requests
- Check the [Roadmap](README.md#roadmap) first
- Use the issue tracker with the "enhancement" label
- Explain the use case and benefits
- Consider implementation complexity

### Code Contributions

#### Getting Started
1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/transcription-assistant.git`
3. Create a feature branch: `git checkout -b feature/amazing-feature`
4. Set up development environment:
   ```bash
   npm install
   npm run tauri:dev
   ```

#### Development Guidelines

**Code Style**
- Follow existing code patterns and conventions
- Use meaningful variable and function names
- Add comments for complex logic
- Keep functions focused and small

**Rust Backend**
- Follow Rust conventions (use `cargo fmt` and `cargo clippy`)
- Handle errors properly with `Result<T, E>`
- Use async/await for I/O operations
- Write unit tests for new functionality

**Frontend (TypeScript)**
- Use TypeScript strict mode
- Follow consistent naming conventions
- Handle promises and async operations properly
- Maintain responsive UI design

#### Testing
- Write tests for new features
- Run existing tests: `cd src-tauri && cargo test`
- Test on multiple platforms when possible
- Verify UI changes across different screen sizes

#### Pull Request Process
1. Update documentation if needed
2. Add entries to CHANGELOG.md for notable changes
3. Ensure all tests pass
4. Create clear, descriptive commit messages
5. Submit PR with:
   - Clear title and description
   - Link to related issues
   - Screenshots for UI changes

## 🏗️ Development Setup

### Prerequisites
- Node.js 16+ and npm/pnpm
- Rust 1.70+
- Platform-specific requirements:
  - **Windows**: Visual Studio Build Tools
  - **macOS**: Xcode Command Line Tools
  - **Linux**: gtk3-devel, webkit2gtk3-devel

### Project Structure
```
transcription-assistant/
├── src/                    # Frontend source
├── src-tauri/             # Rust backend
├── .github/workflows/     # CI/CD pipelines
├── docs/                  # Documentation
└── tests/                 # Integration tests
```

### Available Scripts
```bash
# Development
npm run tauri:dev          # Start with hot reload
npm run dev               # Frontend only
npm run build             # Build frontend

# Testing
npm test                  # Frontend tests
cd src-tauri && cargo test # Backend tests
cargo clippy              # Linting

# Production
npm run tauri:build       # Build application
```

## 🎯 Areas for Contribution

### High Priority
- [ ] Drag & drop file support implementation
- [ ] Batch processing for multiple files
- [ ] Performance optimizations
- [ ] Cross-platform testing

### Medium Priority
- [ ] Built-in Whisper integration
- [ ] Additional export formats
- [ ] UI/UX improvements
- [ ] Documentation improvements

### Low Priority
- [ ] Plugin system architecture
- [ ] Cloud storage integration
- [ ] Advanced audio processing options

## 📋 Code Review Checklist

### Before Submitting
- [ ] Code follows project conventions
- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] No security vulnerabilities introduced
- [ ] Performance impact considered

### Review Criteria
- [ ] Functionality works as expected
- [ ] Code is maintainable and readable
- [ ] Error handling is appropriate
- [ ] UI changes are responsive and accessible
- [ ] No breaking changes without version bump

## 🛡️ Security

### Reporting Vulnerabilities
- **DO NOT** create public issues for security vulnerabilities
- Email security reports to: your-email@example.com
- Include detailed reproduction steps
- Allow reasonable time for response before disclosure

### Security Guidelines
- Never commit secrets or API keys
- Validate all user inputs
- Use secure communication protocols
- Follow OWASP guidelines for web security

## 📄 License

By contributing to Transcription Assistant, you agree that your contributions will be licensed under the MIT License.

## 💬 Community

- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/your-username/transcription-assistant/issues)
- 💡 **Feature Requests**: [GitHub Issues](https://github.com/your-username/transcription-assistant/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/your-username/transcription-assistant/discussions)

## 🎉 Recognition

Contributors will be recognized in:
- GitHub contributors list
- Release notes for significant contributions
- README.md acknowledgments

---

Thank you for helping make Transcription Assistant better! 🚀