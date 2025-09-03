use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Window;
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::audio::{AudioProcessor, ProcessingOptions};
use crate::merger::{TranscriptionMerger, MergeOptions, FileFormat};
use crate::ffmpeg::FFmpegManager;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub duration: String,
    pub size: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessingProgress {
    pub progress: f32,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub success: bool,
    pub output_files: Vec<String>,
    pub message: String,
    pub segments: Vec<SegmentInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentInfo {
    pub path: String,
    pub duration: String,
    pub start_time: f64,
    pub chunk_number: usize,
}

// Global state for merged transcription
lazy_static::lazy_static! {
    static ref MERGED_TRANSCRIPTION: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
}

#[tauri::command]
pub async fn get_file_info(path: String) -> Result<FileInfo, String> {
    println!("Getting file info for path: {}", path);
    let file_path = Path::new(&path);
    
    if !file_path.exists() {
        println!("File does not exist: {}", path);
        return Err(format!("File does not exist: {}", path));
    }

    let metadata = std::fs::metadata(&path).map_err(|e| {
        println!("Failed to get metadata: {}", e);
        format!("Failed to get metadata: {}", e)
    })?;
    
    let file_name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    let size = format_file_size(metadata.len());
    
    // Get duration using FFmpeg
    println!("Attempting to get duration with FFmpeg");
    let duration = match FFmpegManager::new() {
        Ok(ffmpeg_manager) => {
            println!("FFmpegManager created successfully");
            match ffmpeg_manager.get_file_info(&path).await {
                Ok((duration_str, _)) => {
                    println!("Successfully got duration: {}", duration_str);
                    duration_str
                }
                Err(e) => {
                    println!("Failed to get duration: {}", e);
                    "Unknown".to_string()
                }
            }
        }
        Err(e) => {
            println!("Failed to create FFmpegManager: {}", e);
            "Unknown".to_string()
        }
    };
    
    Ok(FileInfo {
        name: file_name,
        duration,
        size,
        path: path.clone(),
    })
}

#[tauri::command]
pub async fn start_audio_processing(
    window: Window,
    file_path: String,
    max_duration: u32,
    use_silence_detection: bool,
) -> Result<ProcessingResult, String> {
    let options = ProcessingOptions {
        max_duration_seconds: max_duration,
        use_silence_detection,
        output_format: "mp3".to_string(),
    };

    let processor = AudioProcessor::new().map_err(|e| e.to_string())?;
    processor.initialize().await.map_err(|e| e.to_string())?;

    let progress_callback = {
        let window = window.clone();
        move |progress: f32, message: String| {
            let _ = window.emit("processing-progress", ProcessingProgress {
                progress,
                message,
            });
        }
    };

    match processor.process_audio_file(&file_path, options, progress_callback).await {
        Ok(chunks) => {
            let output_files: Vec<String> = chunks
                .iter()
                .map(|chunk| chunk.path.to_string_lossy().to_string())
                .collect();

            let segments: Vec<SegmentInfo> = chunks
                .iter()
                .map(|chunk| SegmentInfo {
                    path: chunk.path.to_string_lossy().to_string(),
                    duration: format!("{:.1}s", chunk.duration),
                    start_time: chunk.start_time,
                    chunk_number: chunk.chunk_number,
                })
                .collect();

            let result = ProcessingResult {
                success: true,
                output_files,
                segments,
                message: format!("Successfully created {} audio chunks", chunks.len()),
            };

            let _ = window.emit("processing-complete", &result);
            Ok(result)
        }
        Err(e) => {
            let result = ProcessingResult {
                success: false,
                output_files: vec![],
                segments: vec![],
                message: format!("Processing failed: {}", e),
            };

            let _ = window.emit("processing-complete", &result);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn merge_transcriptions(
    files: Vec<String>,
    output_format: String,
) -> Result<String, String> {
    if files.is_empty() {
        return Err("No transcription files provided".to_string());
    }

    let format = match output_format.to_lowercase().as_str() {
        "srt" => FileFormat::Srt,
        "md" | "markdown" => FileFormat::Markdown,
        _ => FileFormat::Txt,
    };

    let options = MergeOptions {
        output_format: format,
        time_offset_seconds: 0.0,
        remove_timestamps: false,
        add_file_markers: true,
    };

    let mut merger = TranscriptionMerger::new(options);
    
    match merger.add_files(files.clone()).await {
        Ok(_) => {
            match merger.merge().await {
                Ok(merged_content) => {
                    // Store the merged content globally
                    let mut global_transcription = MERGED_TRANSCRIPTION.lock().await;
                    *global_transcription = Some(merged_content.clone());
                    
                    Ok(format!(
                        "Successfully merged {} files ({} segments) into {} format", 
                        merger.get_file_count(),
                        merger.get_total_segments(),
                        output_format
                    ))
                }
                Err(e) => Err(format!("Failed to merge transcriptions: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to load transcription files: {}", e)),
    }
}

#[tauri::command]
pub async fn export_merged_transcription() -> Result<serde_json::Value, String> {
    let global_transcription = MERGED_TRANSCRIPTION.lock().await;
    
    if let Some(content) = global_transcription.as_ref() {
        // Create output file in user's Documents folder
        let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
        let documents_dir = home_dir.join("Documents");
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let output_file = documents_dir.join(format!("transcription_merged_{}.txt", timestamp));
        
        // Write the merged content to file
        std::fs::write(&output_file, content)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        
        let file_path = output_file.to_string_lossy().to_string();
        println!("Exported transcription to: {}", file_path);
        
        Ok(serde_json::json!({
            "path": file_path,
            "size": content.len(),
            "message": format!("Successfully exported {} characters to file", content.len())
        }))
    } else {
        Err("No merged transcription available. Please merge transcriptions first.".to_string())
    }
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}