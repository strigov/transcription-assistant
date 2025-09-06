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
pub async fn get_file_info(window: Window, path: String) -> Result<FileInfo, String> {
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
            // First ensure FFmpeg is available with progress
            match ffmpeg_manager.ensure_ffmpeg_available_with_progress(Some(window.clone())).await {
                Ok(_) => {
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
                    println!("Failed to ensure FFmpeg available: {}", e);
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
pub async fn export_merged_transcription(
    output_path: String,
    file_name: String,
    output_format: String,
    timecode_format: String,
    custom_timecode_format: Option<String>,
    include_extended_info: bool,
) -> Result<serde_json::Value, String> {
    let global_transcription = MERGED_TRANSCRIPTION.lock().await;
    
    if let Some(content) = global_transcription.as_ref() {
        // Build full file path
        let extension = match output_format.as_str() {
            "srt" => "srt",
            "md" => "md", 
            _ => "txt"
        };
        
        let file_name_with_ext = if file_name.contains('.') {
            file_name.clone()
        } else {
            format!("{}.{}", file_name, extension)
        };
        
        let output_file = std::path::Path::new(&output_path).join(&file_name_with_ext);
        
        // Process content based on options
        let processed_content = process_transcription_content(
            content,
            &timecode_format,
            custom_timecode_format.as_deref(),
            include_extended_info,
        )?;
        
        // Write the processed content to file
        std::fs::write(&output_file, &processed_content)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        
        let file_path = output_file.to_string_lossy().to_string();
        println!("Exported transcription to: {}", file_path);
        
        Ok(serde_json::json!({
            "path": file_path,
            "size": processed_content.len(),
            "message": format!("Successfully exported {} characters to file", processed_content.len())
        }))
    } else {
        Err("No merged transcription available. Please merge transcriptions first.".to_string())
    }
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    println!("Opening folder: {}", path);
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Use cmd /c start to handle paths with special characters better
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
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

fn process_transcription_content(
    content: &str,
    timecode_format: &str,
    custom_format: Option<&str>,
    include_extended_info: bool,
) -> Result<String, String> {
    use regex::Regex;
    
    // Parse and process each line of the transcription
    let mut processed_lines = Vec::new();
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            processed_lines.push(String::new());
            continue;
        }
        
        // Try to match different formats that merger might create
        
        // Format 1: [timecode] [something] [maybe_another_timecode] text  
        // This handles cases like: [00:00:00] [filename] [00:00] text
        let re_complex = Regex::new(r"^\[(\d{1,2}:\d{2}(?::\d{2})?|\d+)\]\s*\[([^\]]+)\]\s*(?:\[([^\]]+)\]\s*)?(.*)$")
            .map_err(|e| format!("Regex error: {}", e))?;
            
        // Format 2: [timecode] [something] text (two brackets)
        let re_with_file = Regex::new(r"^\[(\d{1,2}:\d{2}(?::\d{2})?|\d+)\]\s*\[([^\]]+)\]\s*(.*)$")
            .map_err(|e| format!("Regex error: {}", e))?;
            
        // Format 3: [timecode] text (simple format)  
        let re_simple = Regex::new(r"^\[(\d{1,2}:\d{2}(?::\d{2})?|\d+)\]\s*(.*)$")
            .map_err(|e| format!("Regex error: {}", e))?;
        
        if let Some(captures) = re_complex.captures(line) {
            // Format: [timecode] [info1] [info2] text or [timecode] [info1] text
            let current_timecode = captures.get(1).unwrap().as_str();
            let info1 = captures.get(2).unwrap().as_str();
            let info2 = captures.get(3).map(|m| m.as_str());
            let text = captures.get(4).unwrap().as_str();
            
            // Convert timecode to requested format
            let formatted_timecode = convert_timecode(current_timecode, timecode_format, custom_format)?;
            
            // Build the line based on extended info option
            let processed_line = if include_extended_info {
                // Include extended info
                if let Some(info2_val) = info2 {
                    format!("[{}] [{} {}] {}", formatted_timecode, info1, info2_val, text)
                } else {
                    format!("[{}] [{}] {}", formatted_timecode, info1, text)
                }
            } else {
                // Remove all extra info - user doesn't want extended info
                format!("[{}] {}", formatted_timecode, text)
            };
            
            processed_lines.push(processed_line);
        } else if let Some(captures) = re_with_file.captures(line) {
            // Format: [timecode] [info] text
            let current_timecode = captures.get(1).unwrap().as_str();
            let info = captures.get(2).unwrap().as_str();
            let text = captures.get(3).unwrap().as_str();
            
            // Convert timecode to requested format
            let formatted_timecode = convert_timecode(current_timecode, timecode_format, custom_format)?;
            
            // Build the line based on extended info option
            let processed_line = if include_extended_info {
                format!("[{}] [{}] {}", formatted_timecode, info, text)
            } else {
                // Remove extra info
                format!("[{}] {}", formatted_timecode, text)
            };
            
            processed_lines.push(processed_line);
        } else if let Some(captures) = re_simple.captures(line) {
            // Format: [timecode] text
            let current_timecode = captures.get(1).unwrap().as_str();
            let text = captures.get(2).unwrap().as_str();
            
            // Convert timecode to requested format
            let formatted_timecode = convert_timecode(current_timecode, timecode_format, custom_format)?;
            
            // Simple format
            let processed_line = format!("[{}] {}", formatted_timecode, text);
            processed_lines.push(processed_line);
        } else {
            // If line doesn't match expected format, keep as is
            processed_lines.push(line.to_string());
        }
    }
    
    Ok(processed_lines.join("\n"))
}

fn convert_timecode(
    timecode: &str,
    target_format: &str,
    custom_format: Option<&str>,
) -> Result<String, String> {
    // Parse various time formats to total seconds
    let total_seconds = parse_timecode_to_seconds(timecode)?;
    
    match target_format {
        "hms" => {
            // Convert to HH:MM:SS format
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            Ok(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
        },
        "hms_ms" => {
            // Convert to HH:MM:SS.000 format (no milliseconds available, so .000)
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            Ok(format!("{:02}:{:02}:{:02}.000", hours, minutes, seconds))
        },
        "seconds" => {
            // Just total seconds
            Ok(total_seconds.to_string())
        },
        "seconds_ms" => {
            // Seconds with .0 (no milliseconds available)
            Ok(format!("{}.0", total_seconds))
        },
        "custom" => {
            if let Some(custom_fmt) = custom_format {
                // Simple custom format processing
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                let seconds = total_seconds % 60;
                
                let result = custom_fmt
                    .replace("HH", &format!("{:02}", hours))
                    .replace("MM", &format!("{:02}", minutes))
                    .replace("SS", &format!("{:02}", seconds))
                    .replace("MS", "000"); // No milliseconds available
                    
                Ok(result)
            } else {
                Err("Custom format specified but no format provided".to_string())
            }
        },
        _ => {
            // Default: keep original MM:SS format
            Ok(timecode.to_string())
        }
    }
}

fn parse_timecode_to_seconds(timecode: &str) -> Result<u32, String> {
    let parts: Vec<&str> = timecode.split(':').collect();
    
    match parts.len() {
        2 => {
            // MM:SS format
            let minutes: u32 = parts[0].parse().map_err(|_| "Invalid minutes")?;
            let seconds: u32 = parts[1].parse().map_err(|_| "Invalid seconds")?;
            Ok(minutes * 60 + seconds)
        },
        3 => {
            // HH:MM:SS format
            let hours: u32 = parts[0].parse().map_err(|_| "Invalid hours")?;
            let minutes: u32 = parts[1].parse().map_err(|_| "Invalid minutes")?;
            let seconds: u32 = parts[2].parse().map_err(|_| "Invalid seconds")?;
            Ok(hours * 3600 + minutes * 60 + seconds)
        },
        1 => {
            // Maybe just seconds (e.g., "330")
            let seconds: u32 = parts[0].parse().map_err(|_| "Invalid seconds")?;
            Ok(seconds)
        },
        _ => {
            Err(format!("Unsupported timecode format: {}", timecode))
        }
    }
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