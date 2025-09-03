use anyhow::{anyhow, Result};
use reqwest;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

pub struct FFmpegManager {
    ffmpeg_path: PathBuf,
    app_data_dir: PathBuf,
}

impl FFmpegManager {
    pub fn new() -> Result<Self> {
        let app_data_dir = get_app_data_dir()?;
        let ffmpeg_dir = app_data_dir.join("ffmpeg");
        
        #[cfg(target_os = "windows")]
        let ffmpeg_path = ffmpeg_dir.join("ffmpeg.exe");
        #[cfg(not(target_os = "windows"))]
        let ffmpeg_path = ffmpeg_dir.join("ffmpeg");

        Ok(Self {
            ffmpeg_path,
            app_data_dir,
        })
    }

    pub async fn ensure_ffmpeg_available(&self) -> Result<()> {
        if self.is_ffmpeg_available().await {
            return Ok(());
        }

        // Try to find system FFmpeg first
        if self.find_system_ffmpeg().is_some() {
            return Ok(());
        }

        // Download and install FFmpeg
        self.download_ffmpeg().await?;
        Ok(())
    }

    pub async fn is_ffmpeg_available(&self) -> bool {
        if self.ffmpeg_path.exists() {
            return self.test_ffmpeg(&self.ffmpeg_path).await;
        }

        if let Some(system_path) = self.find_system_ffmpeg() {
            return self.test_ffmpeg(&system_path).await;
        }

        false
    }

    pub fn get_ffmpeg_path(&self) -> Result<PathBuf> {
        if self.ffmpeg_path.exists() {
            return Ok(self.ffmpeg_path.clone());
        }

        if let Some(system_path) = self.find_system_ffmpeg() {
            return Ok(system_path);
        }

        Err(anyhow!("FFmpeg not found"))
    }

    fn find_system_ffmpeg(&self) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let command = "ffmpeg.exe";
        #[cfg(not(target_os = "windows"))]
        let command = "ffmpeg";

        if let Ok(output) = Command::new("which").arg(command).output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let trimmed_path = path_str.trim();
                return Some(PathBuf::from(trimmed_path));
            }
        }

        // Check common installation paths
        #[cfg(target_os = "windows")]
        let common_paths = vec![
            PathBuf::from("C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe"),
            PathBuf::from("C:\\ffmpeg\\bin\\ffmpeg.exe"),
        ];

        #[cfg(target_os = "macos")]
        let common_paths = vec![
            PathBuf::from("/usr/local/bin/ffmpeg"),
            PathBuf::from("/opt/homebrew/bin/ffmpeg"),
            PathBuf::from("/usr/bin/ffmpeg"),
        ];

        #[cfg(target_os = "linux")]
        let common_paths = vec![
            PathBuf::from("/usr/bin/ffmpeg"),
            PathBuf::from("/usr/local/bin/ffmpeg"),
        ];

        for path in common_paths {
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    async fn test_ffmpeg(&self, path: &Path) -> bool {
        match Command::new(path).arg("-version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    async fn download_ffmpeg(&self) -> Result<()> {
        let download_url = self.get_download_url();
        let ffmpeg_dir = self.ffmpeg_path.parent().unwrap();
        
        // Create directory
        fs::create_dir_all(ffmpeg_dir).await?;

        // Download FFmpeg
        println!("Downloading FFmpeg from: {}", download_url);
        let response = reqwest::get(&download_url).await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download FFmpeg: HTTP {}", response.status()));
        }

        let bytes = response.bytes().await?;
        let archive_path = ffmpeg_dir.join("ffmpeg.zip");
        
        // Save zip file
        let mut file = fs::File::create(&archive_path).await?;
        file.write_all(&bytes).await?;
        file.sync_all().await?;
        drop(file);

        // Extract zip
        self.extract_ffmpeg(&archive_path).await?;
        
        // Clean up zip file
        fs::remove_file(archive_path).await?;

        println!("FFmpeg installed successfully");
        Ok(())
    }

    fn get_download_url(&self) -> String {
        #[cfg(target_os = "windows")]
        return "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip".to_string();
        
        #[cfg(target_os = "macos")]
        return "https://evermeet.cx/ffmpeg/ffmpeg-6.0.zip".to_string();
        
        #[cfg(target_os = "linux")]
        return "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz".to_string();
    }

    async fn extract_ffmpeg(&self, archive_path: &Path) -> Result<()> {
        let file = std::fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_path = file.mangled_name();
            
            if file_path.file_name().unwrap_or_default() == "ffmpeg" 
                || file_path.file_name().unwrap_or_default() == "ffmpeg.exe" {
                
                let target_path = &self.ffmpeg_path;
                let mut target_file = std::fs::File::create(target_path)?;
                std::io::copy(&mut file, &mut target_file)?;
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(target_path)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(target_path, perms)?;
                }
                
                break;
            }
        }
        
        Ok(())
    }

    pub async fn get_file_info(&self, file_path: &str) -> Result<(String, f64)> {
        // First ensure FFmpeg is available
        self.ensure_ffmpeg_available().await?;
        
        let ffmpeg_path = self.get_ffmpeg_path()?;
        
        println!("Using FFmpeg at: {:?}", ffmpeg_path);
        println!("Getting info for file: {}", file_path);
        
        // Check if file exists first
        if !std::path::Path::new(file_path).exists() {
            return Err(anyhow!("File does not exist: {}", file_path));
        }
        
        let output = Command::new(&ffmpeg_path)
            .args([
                "-i", file_path,
                "-v", "error",  // Change from quiet to error to get more info
                "-f", "null", "-"
            ])
            .output()?;

        // FFmpeg outputs file info to stderr, check both stderr and stdout
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let combined_output = format!("{}\n{}", stderr_str, stdout_str);
        
        println!("FFmpeg stderr: {}", stderr_str);
        println!("FFmpeg stdout: {}", stdout_str);
        
        if !output.status.success() {
            println!("FFmpeg failed with status: {:?}", output.status);
            return Err(anyhow!("Failed to get file info: {}", stderr_str));
        }

        // Parse duration from the Duration line in either output
        let duration = if let Some(duration_line) = combined_output.lines()
            .find(|line| line.trim().starts_with("Duration:")) {
            
            println!("Found duration line: {}", duration_line);
            if let Some(duration_part) = duration_line.split("Duration:").nth(1) {
                if let Some(time_part) = duration_part.split(',').next() {
                    let parsed = parse_duration_string(time_part.trim()).unwrap_or(0.0);
                    println!("Parsed duration: {} seconds", parsed);
                    parsed
                } else {
                    println!("Could not split time part");
                    0.0
                }
            } else {
                println!("Could not split duration part");
                0.0
            }
        } else {
            println!("No Duration line found in output");
            // Try alternative approach: run FFmpeg with -hide_banner for cleaner output
            self.get_file_info_alternative(file_path).await.unwrap_or(0.0)
        };
        
        Ok((format_duration(duration), duration))
    }

    async fn get_file_info_alternative(&self, file_path: &str) -> Result<f64> {
        let ffmpeg_path = self.get_ffmpeg_path()?;
        
        println!("Trying alternative approach with -hide_banner");
        let output = Command::new(&ffmpeg_path)
            .args([
                "-hide_banner",
                "-i", file_path,
                "-f", "null", "-"
            ])
            .output()?;

        let stderr_str = String::from_utf8_lossy(&output.stderr);
        println!("Alternative FFmpeg output: {}", stderr_str);
        
        if let Some(duration_line) = stderr_str.lines()
            .find(|line| line.trim().contains("Duration:")) {
            
            if let Some(duration_part) = duration_line.split("Duration:").nth(1) {
                if let Some(time_part) = duration_part.split(',').next() {
                    return Ok(parse_duration_string(time_part.trim()).unwrap_or(0.0));
                }
            }
        }
        
        Ok(0.0)
    }
}

fn parse_duration_string(duration_str: &str) -> Result<f64> {
    // Parse duration in format HH:MM:SS.sss
    let parts: Vec<&str> = duration_str.split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow!("Invalid duration format"));
    }
    
    let hours: f64 = parts[0].parse()?;
    let minutes: f64 = parts[1].parse()?;
    let seconds: f64 = parts[2].parse()?;
    
    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}

fn get_app_data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(appdata).join("TranscriptionAssistant"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("TranscriptionAssistant"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home).join(".config").join("transcription-assistant"));
        }
    }

    Err(anyhow!("Could not determine app data directory"))
}

fn format_duration(seconds: f64) -> String {
    let total_seconds = seconds as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;
    
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}