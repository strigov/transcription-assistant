use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub text: String,
    pub file_index: usize,
    pub original_filename: String,
}

#[derive(Debug, Clone)]
pub struct TranscriptionFile {
    pub path: PathBuf,
    pub filename: String,
    pub sequence_number: Option<usize>,
    pub format: FileFormat,
    pub segments: Vec<TranscriptionSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileFormat {
    Txt,
    Srt,
    Markdown,
}

#[derive(Debug, Clone)]
pub struct MergeOptions {
    pub output_format: FileFormat,
    pub time_offset_seconds: f64,
    pub remove_timestamps: bool,
    pub add_file_markers: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            output_format: FileFormat::Txt,
            time_offset_seconds: 0.0,
            remove_timestamps: false,
            add_file_markers: true,
        }
    }
}

pub struct TranscriptionMerger {
    files: Vec<TranscriptionFile>,
    merge_options: MergeOptions,
}

impl TranscriptionMerger {
    pub fn new(options: MergeOptions) -> Self {
        Self {
            files: Vec::new(),
            merge_options: options,
        }
    }

    pub async fn add_files(&mut self, file_paths: Vec<String>) -> Result<()> {
        for path_str in file_paths {
            let path = PathBuf::from(&path_str);
            let file = self.parse_transcription_file(&path).await?;
            self.files.push(file);
        }

        // Sort files by sequence number
        self.files.sort_by_key(|f| f.sequence_number.unwrap_or(999999));
        
        Ok(())
    }

    async fn parse_transcription_file(&self, path: &Path) -> Result<TranscriptionFile> {
        let content = fs::read_to_string(path).await?;
        let filename = path.file_name()
            .ok_or_else(|| anyhow!("Invalid filename"))?
            .to_string_lossy()
            .to_string();

        let format = self.detect_format(path, &content)?;
        let sequence_number = self.extract_sequence_number(&filename);

        let segments = match format {
            FileFormat::Srt => self.parse_srt(&content, &filename)?,
            FileFormat::Txt => self.parse_txt(&content, &filename)?,
            FileFormat::Markdown => self.parse_markdown(&content, &filename)?,
        };

        Ok(TranscriptionFile {
            path: path.to_path_buf(),
            filename,
            sequence_number,
            format,
            segments,
        })
    }

    fn detect_format(&self, path: &Path, content: &str) -> Result<FileFormat> {
        if let Some(ext) = path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "srt" => return Ok(FileFormat::Srt),
                "md" => return Ok(FileFormat::Markdown),
                "txt" => {
                    // Check if it's actually SRT format
                    if self.looks_like_srt(content) {
                        return Ok(FileFormat::Srt);
                    }
                    return Ok(FileFormat::Txt);
                }
                _ => {}
            }
        }

        // Fallback to content-based detection
        if self.looks_like_srt(content) {
            Ok(FileFormat::Srt)
        } else if content.contains("# ") || content.contains("## ") {
            Ok(FileFormat::Markdown)
        } else {
            Ok(FileFormat::Txt)
        }
    }

    fn looks_like_srt(&self, content: &str) -> bool {
        let srt_pattern = Regex::new(r"\d+\s*\n\d{2}:\d{2}:\d{2}[,\.]\d{3} --> \d{2}:\d{2}:\d{2}[,\.]\d{3}").unwrap();
        srt_pattern.is_match(content)
    }

    fn extract_sequence_number(&self, filename: &str) -> Option<usize> {
        let patterns = [
            r"(\d+)", // Any number
            r"chunk[_-]?(\d+)",
            r"part[_-]?(\d+)",
            r"segment[_-]?(\d+)",
        ];

        for pattern in patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(filename) {
                    if let Some(num_str) = captures.get(1) {
                        if let Ok(num) = num_str.as_str().parse::<usize>() {
                            return Some(num);
                        }
                    }
                }
            }
        }

        None
    }

    fn parse_srt(&self, content: &str, filename: &str) -> Result<Vec<TranscriptionSegment>> {
        let mut segments = Vec::new();
        let blocks: Vec<&str> = content.split("\n\n").collect();

        for (index, block) in blocks.iter().enumerate() {
            let lines: Vec<&str> = block.trim().lines().collect();
            if lines.len() < 3 {
                continue;
            }

            // Parse timestamp line (format: 00:00:00,000 --> 00:00:01,000)
            let timestamp_line = lines[1];
            if let Some((start_str, end_str)) = timestamp_line.split_once(" --> ") {
                let start_time = self.parse_srt_timestamp(start_str)?;
                let end_time = Some(self.parse_srt_timestamp(end_str)?);
                
                // Join remaining lines as text
                let text = lines[2..].join(" ").trim().to_string();
                
                if !text.is_empty() {
                    segments.push(TranscriptionSegment {
                        start_time,
                        end_time,
                        text,
                        file_index: index,
                        original_filename: filename.to_string(),
                    });
                }
            }
        }

        Ok(segments)
    }

    fn parse_srt_timestamp(&self, timestamp_str: &str) -> Result<f64> {
        // Parse format: 00:00:00,000 or 00:00:00.000
        let normalized = timestamp_str.replace(',', ".");
        let parts: Vec<&str> = normalized.split(':').collect();
        
        if parts.len() != 3 {
            return Err(anyhow!("Invalid timestamp format: {}", timestamp_str));
        }

        let hours: f64 = parts[0].parse()?;
        let minutes: f64 = parts[1].parse()?;
        let seconds: f64 = parts[2].parse()?;

        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    }

    fn parse_txt(&self, content: &str, filename: &str) -> Result<Vec<TranscriptionSegment>> {
        let mut segments = Vec::new();
        
        // Try to find timestamps in the text
        let timestamp_regex = Regex::new(r"\[(\d{1,2}):(\d{2}):(\d{2}(?:\.\d{1,3})?)\]|(\d{1,2}):(\d{2}):(\d{2}(?:\.\d{1,3})?)").unwrap();
        
        let lines: Vec<&str> = content.lines().collect();
        let mut current_time = 0.0;
        let average_read_speed = 150.0; // words per minute

        for (index, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut segment_start_time = current_time;
            let mut text = line.to_string();

            // Check if line contains timestamp
            if let Some(captures) = timestamp_regex.captures(line) {
                if let Some(h) = captures.get(1) {
                    // Format: [HH:MM:SS.mmm]
                    let hours: f64 = h.as_str().parse().unwrap_or(0.0);
                    let minutes: f64 = captures.get(2).unwrap().as_str().parse().unwrap_or(0.0);
                    let seconds: f64 = captures.get(3).unwrap().as_str().parse().unwrap_or(0.0);
                    segment_start_time = hours * 3600.0 + minutes * 60.0 + seconds;
                    current_time = segment_start_time;
                    
                    // Remove timestamp from text
                    text = timestamp_regex.replace(&text, "").trim().to_string();
                } else if let Some(h) = captures.get(4) {
                    // Format: HH:MM:SS.mmm (without brackets)
                    let hours: f64 = h.as_str().parse().unwrap_or(0.0);
                    let minutes: f64 = captures.get(5).unwrap().as_str().parse().unwrap_or(0.0);
                    let seconds: f64 = captures.get(6).unwrap().as_str().parse().unwrap_or(0.0);
                    segment_start_time = hours * 3600.0 + minutes * 60.0 + seconds;
                    current_time = segment_start_time;
                    
                    text = timestamp_regex.replace(&text, "").trim().to_string();
                }
            }

            if !text.is_empty() {
                // Estimate duration based on word count
                let word_count = text.split_whitespace().count();
                let estimated_duration = (word_count as f64 / average_read_speed) * 60.0;
                current_time += estimated_duration.max(1.0);

                segments.push(TranscriptionSegment {
                    start_time: segment_start_time,
                    end_time: Some(current_time),
                    text,
                    file_index: index,
                    original_filename: filename.to_string(),
                });
            }
        }

        // If no segments found, treat entire content as one segment
        if segments.is_empty() && !content.trim().is_empty() {
            segments.push(TranscriptionSegment {
                start_time: 0.0,
                end_time: None,
                text: content.trim().to_string(),
                file_index: 0,
                original_filename: filename.to_string(),
            });
        }

        Ok(segments)
    }

    fn parse_markdown(&self, content: &str, filename: &str) -> Result<Vec<TranscriptionSegment>> {
        let mut segments = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_time = 0.0;

        for (index, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Estimate timing based on content
            let word_count = line.split_whitespace().count();
            let estimated_duration = (word_count as f64 / 150.0) * 60.0; // 150 WPM
            
            segments.push(TranscriptionSegment {
                start_time: current_time,
                end_time: Some(current_time + estimated_duration.max(1.0)),
                text: line.to_string(),
                file_index: index,
                original_filename: filename.to_string(),
            });

            current_time += estimated_duration.max(1.0);
        }

        Ok(segments)
    }

    pub async fn merge(&self) -> Result<String> {
        let mut all_segments = Vec::new();
        let mut cumulative_offset = self.merge_options.time_offset_seconds;

        for (file_index, file) in self.files.iter().enumerate() {
            for mut segment in file.segments.clone() {
                // Apply time offset
                segment.start_time += cumulative_offset;
                if let Some(end_time) = segment.end_time {
                    segment.end_time = Some(end_time + cumulative_offset);
                }
                
                all_segments.push(segment);
            }

            // Add gap between files (estimated based on last segment)
            if file_index < self.files.len() - 1 {
                if let Some(last_segment) = file.segments.last() {
                    let file_duration = last_segment.end_time.unwrap_or(last_segment.start_time + 30.0);
                    cumulative_offset += file_duration;
                }
            }
        }

        // Sort by start time
        all_segments.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        match self.merge_options.output_format {
            FileFormat::Srt => self.format_as_srt(&all_segments),
            FileFormat::Txt => self.format_as_txt(&all_segments),
            FileFormat::Markdown => self.format_as_markdown(&all_segments),
        }
    }

    fn format_as_srt(&self, segments: &[TranscriptionSegment]) -> Result<String> {
        let mut output = String::new();

        for (index, segment) in segments.iter().enumerate() {
            output.push_str(&format!("{}\n", index + 1));
            
            let start = self.format_srt_timestamp(segment.start_time);
            let end = if let Some(end_time) = segment.end_time {
                self.format_srt_timestamp(end_time)
            } else {
                self.format_srt_timestamp(segment.start_time + 5.0) // Default 5 second duration
            };
            
            output.push_str(&format!("{} --> {}\n", start, end));
            
            if self.merge_options.add_file_markers {
                output.push_str(&format!("[{}] {}\n\n", segment.original_filename, segment.text));
            } else {
                output.push_str(&format!("{}\n\n", segment.text));
            }
        }

        Ok(output)
    }

    fn format_as_txt(&self, segments: &[TranscriptionSegment]) -> Result<String> {
        let mut output = String::new();

        for segment in segments {
            if !self.merge_options.remove_timestamps {
                let timestamp = self.format_txt_timestamp(segment.start_time);
                output.push_str(&format!("[{}] ", timestamp));
            }
            
            if self.merge_options.add_file_markers {
                output.push_str(&format!("[{}] ", segment.original_filename));
            }
            
            output.push_str(&format!("{}\n", segment.text));
        }

        Ok(output)
    }

    fn format_as_markdown(&self, segments: &[TranscriptionSegment]) -> Result<String> {
        let mut output = String::new();
        output.push_str("# Merged Transcription\n\n");
        
        let now: DateTime<Utc> = Utc::now();
        output.push_str(&format!("*Generated on: {}*\n\n", now.format("%Y-%m-%d %H:%M:%S UTC")));

        let mut current_file = String::new();
        
        for segment in segments {
            if self.merge_options.add_file_markers && segment.original_filename != current_file {
                current_file = segment.original_filename.clone();
                output.push_str(&format!("## {}\n\n", current_file));
            }
            
            if !self.merge_options.remove_timestamps {
                let timestamp = self.format_txt_timestamp(segment.start_time);
                output.push_str(&format!("**[{}]** ", timestamp));
            }
            
            output.push_str(&format!("{}\n\n", segment.text));
        }

        Ok(output)
    }

    fn format_srt_timestamp(&self, seconds: f64) -> String {
        let total_seconds = seconds as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;
        let millis = ((seconds - total_seconds as f64) * 1000.0) as u32;
        
        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, secs, millis)
    }

    fn format_txt_timestamp(&self, seconds: f64) -> String {
        let total_seconds = seconds as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;
        
        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{:02}:{:02}", minutes, secs)
        }
    }

    pub fn get_file_count(&self) -> usize {
        self.files.len()
    }

    pub fn get_total_segments(&self) -> usize {
        self.files.iter().map(|f| f.segments.len()).sum()
    }
}