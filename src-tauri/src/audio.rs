use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use crate::ffmpeg::FFmpegManager;

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub path: PathBuf,
    pub start_time: f64,
    pub duration: f64,
    pub chunk_number: usize,
}

#[derive(Debug)]
pub struct ProcessingOptions {
    pub max_duration_seconds: u32,
    pub use_silence_detection: bool,
    pub output_format: String,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            max_duration_seconds: 1800, // 30 minutes
            use_silence_detection: true,
            output_format: "mp3".to_string(),
        }
    }
}

pub struct AudioProcessor {
    ffmpeg_manager: FFmpegManager,
}

impl AudioProcessor {
    pub fn new() -> Result<Self> {
        let ffmpeg_manager = FFmpegManager::new()?;
        
        Ok(Self {
            ffmpeg_manager,
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        // Ensure FFmpeg is available
        self.ffmpeg_manager.ensure_ffmpeg_available().await?;
        
        Ok(())
    }

    pub async fn process_audio_file(
        &self,
        input_path: &str,
        options: ProcessingOptions,
        progress_callback: impl Fn(f32, String) + Clone,
    ) -> Result<Vec<AudioChunk>> {
        println!("Starting audio processing for: {}", input_path);
        progress_callback(0.0, "Анализ аудиофайла...".to_string());
        
        // Get file info
        let (_duration_str, total_duration) = self.ffmpeg_manager.get_file_info(input_path).await?;
        println!("Total duration: {} seconds", total_duration);
        
        if total_duration == 0.0 {
            return Err(anyhow!("Could not determine file duration"));
        }
        
        // Create output directory next to the source file
        let input_path_buf = Path::new(input_path);
        let output_dir = if let Some(parent) = input_path_buf.parent() {
            let file_stem = input_path_buf.file_stem().unwrap_or_default().to_string_lossy();
            parent.join(format!("{}_segments", file_stem))
        } else {
            Path::new(".").join("audio_segments")
        };
        
        fs::create_dir_all(&output_dir).await?;
        println!("Created output directory: {:?}", output_dir);
        
        progress_callback(10.0, "Планирование разделения аудио...".to_string());
        
        let chunks = if options.use_silence_detection {
            println!("Using silence detection for splitting");
            self.split_by_silence(input_path, &options, total_duration, &output_dir, progress_callback.clone()).await?
        } else {
            println!("Using time-based splitting");
            self.split_by_time(input_path, &options, total_duration, &output_dir, progress_callback.clone()).await?
        };
        
        println!("Created {} chunks", chunks.len());
        progress_callback(100.0, "Обработка аудио завершена!".to_string());
        
        Ok(chunks)
    }

    async fn split_by_time(
        &self,
        input_path: &str,
        options: &ProcessingOptions,
        total_duration: f64,
        output_dir: &Path,
        progress_callback: impl Fn(f32, String),
    ) -> Result<Vec<AudioChunk>> {
        let max_duration = options.max_duration_seconds as f64;
        let chunk_count = (total_duration / max_duration).ceil() as usize;
        let mut chunks = Vec::new();

        for i in 0..chunk_count {
            let start_time = i as f64 * max_duration;
            let duration = if start_time + max_duration > total_duration {
                total_duration - start_time
            } else {
                max_duration
            };

            progress_callback(
                20.0 + (70.0 * (i as f32 + 1.0) / chunk_count as f32),
                format!("Обработка сегмента {} из {}...", i + 1, chunk_count),
            );

            let chunk_path = output_dir.join(format!("chunk_{:03}.{}", i + 1, options.output_format));
            
            self.extract_audio_segment(input_path, &chunk_path, start_time, duration).await?;

            chunks.push(AudioChunk {
                path: chunk_path,
                start_time,
                duration,
                chunk_number: i + 1,
            });
        }

        Ok(chunks)
    }

    async fn split_by_silence(
        &self,
        input_path: &str,
        options: &ProcessingOptions,
        total_duration: f64,
        output_dir: &Path,
        progress_callback: impl Fn(f32, String),
    ) -> Result<Vec<AudioChunk>> {
        progress_callback(15.0, "Поиск точек тишины...".to_string());
        
        // Detect silence points
        let silence_points = self.detect_silence_points(input_path).await?;
        println!("Found {} silence points: {:?}", silence_points.len(), silence_points);
        
        // If no silence points found or very few, fallback to time-based splitting
        if silence_points.len() < 2 {
            println!("Not enough silence points found, falling back to time-based splitting");
            return self.split_by_time(input_path, options, total_duration, output_dir, progress_callback).await;
        }
        
        progress_callback(25.0, "Создание сегментов на основе тишины...".to_string());
        
        let mut chunks = Vec::new();
        let mut current_start = 0.0;
        let mut chunk_number = 1;
        let max_duration = options.max_duration_seconds as f64;

        for (i, &silence_point) in silence_points.iter().enumerate() {
            let current_duration = silence_point - current_start;
            
            // If this chunk would be too long, or we've reached the end
            if current_duration >= max_duration || i == silence_points.len() - 1 {
                progress_callback(
                    25.0 + (65.0 * (chunk_number as f32) / (silence_points.len() as f32 + 1.0)),
                    format!("Обработка сегмента {}...", chunk_number),
                );

                let end_time = if i == silence_points.len() - 1 { total_duration } else { silence_point };
                let actual_duration = end_time - current_start;

                let chunk_path = output_dir.join(format!("chunk_{:03}.{}", chunk_number, options.output_format));
                
                self.extract_audio_segment(input_path, &chunk_path, current_start, actual_duration).await?;

                chunks.push(AudioChunk {
                    path: chunk_path,
                    start_time: current_start,
                    duration: actual_duration,
                    chunk_number,
                });

                current_start = silence_point;
                chunk_number += 1;
            }
        }

        // Handle case where no silence was detected
        if chunks.is_empty() {
            return self.split_by_time(input_path, options, total_duration, output_dir, progress_callback).await;
        }

        Ok(chunks)
    }

    async fn detect_silence_points(&self, input_path: &str) -> Result<Vec<f64>> {
        println!("Detecting silence points in: {}", input_path);
        let ffmpeg_path = self.ffmpeg_manager.get_ffmpeg_path()?;
        
        let mut cmd = Command::new(&ffmpeg_path);
        cmd.args([
            "-i", input_path,
            "-af", "silencedetect=noise=-40dB:duration=1",  // More sensitive settings
            "-f", "null",
            "-",
            "-v", "info",
        ]);
        
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        
        let output = cmd.output()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("FFmpeg silence detection output: {}", stderr);
        
        let mut silence_points = Vec::new();

        for line in stderr.lines() {
            if line.contains("silence_end") {
                println!("Found silence_end line: {}", line);
                if let Some(time_str) = extract_time_from_silence_line(line) {
                    if let Ok(time) = time_str.parse::<f64>() {
                        println!("Parsed silence point: {}", time);
                        silence_points.push(time);
                    }
                }
            }
        }

        // Sort and deduplicate
        silence_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        silence_points.dedup_by(|a, b| (*a - *b).abs() < 0.1);

        Ok(silence_points)
    }

    async fn extract_audio_segment(
        &self,
        input_path: &str,
        output_path: &Path,
        start_time: f64,
        duration: f64,
    ) -> Result<()> {
        println!("Extracting segment: start={}, duration={}, output={:?}", start_time, duration, output_path);
        
        // Ensure temp directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let ffmpeg_path = self.ffmpeg_manager.get_ffmpeg_path()?;
        
        let mut cmd = Command::new(ffmpeg_path);
        cmd.args([
            "-i", input_path,
            "-ss", &start_time.to_string(),
            "-t", &duration.to_string(),
            "-acodec", "libmp3lame",  // MP3 encoder
            "-b:a", "128k",           // 128 kbps bitrate
            "-ar", "44100",           // Keep original sample rate
            "-ac", "2",               // Keep stereo
            "-y",
            output_path.to_str().unwrap(),
        ]);
        
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        
        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("FFmpeg extraction failed: {}", stderr);
            return Err(anyhow!("FFmpeg failed: {}", stderr));
        }
        
        println!("Successfully extracted segment to: {:?}", output_path);

        Ok(())
    }
}

fn extract_time_from_silence_line(line: &str) -> Option<String> {
    // Parse lines like: "[silencedetect @ 0x...] silence_end: 123.456 | silence_duration: 2.345"
    if let Some(pos) = line.find("silence_end: ") {
        let after_label = &line[pos + 13..];
        if let Some(end_pos) = after_label.find(' ') {
            return Some(after_label[..end_pos].to_string());
        }
    }
    None
}