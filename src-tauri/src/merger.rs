use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use encoding_rs::WINDOWS_1251;
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
#[allow(dead_code)]
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
        let raw_bytes = fs::read(path).await?;
        let content = read_text_with_encoding(&raw_bytes);
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
        // Handle both LF and CRLF line endings
        let srt_pattern = Regex::new(r"\d+\s*\r?\n\d{2}:\d{2}:\d{2}[,\.]\d{3} --> \d{2}:\d{2}:\d{2}[,\.]\d{3}").unwrap();
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
        // Normalize CRLF to LF before splitting to handle Windows-encoded SRT files
        let normalized = content.replace("\r\n", "\n");
        let blocks: Vec<&str> = normalized.split("\n\n").collect();

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

        // Range timestamp patterns (e.g., [00:00-01:06] or [01:30:00-01:31:25])
        // These MUST be checked first, before single-timestamp patterns
        let range_hh_mm_ss = Regex::new(
            r"\[(\d{1,2}):(\d{2}):(\d{2})-(\d{1,2}):(\d{2}):(\d{2})\]"
        ).unwrap();
        let range_mm_ss = Regex::new(
            r"\[(\d{1,2}):(\d{2})-(\d{1,2}):(\d{2})\]"
        ).unwrap();

        // Multiple regex patterns for different single-timecode formats
        let patterns = [
            // [HH:MM:SS.mmm] format - full precision with brackets
            r"\[(\d{1,2}):(\d{2}):(\d{2})(?:[\.,](\d{1,3}))?\]",
            // [MM:SS] format - minutes:seconds with brackets
            r"\[(\d{1,2}):(\d{2})\]",
            // HH:MM:SS.mmm format - full precision without brackets
            r"^(\d{1,2}):(\d{2}):(\d{2})(?:[\.,](\d{1,3}))?(?:\s|$)",
            // MM:SS format - minutes:seconds without brackets
            r"^(\d{1,2}):(\d{2})(?:\s|$)",
            // Whisper format: [HH:MM:SS.mmm --> HH:MM:SS.mmm] (extract start time)
            r"\[(\d{1,2}):(\d{2}):(\d{2})(?:[\.,](\d{1,3}))?\s*-->\s*\d{1,2}:\d{2}:\d{2}(?:[\.,]\d{1,3})?\]",
            // Simple seconds format: [123] (only bracketed, to avoid catching plain numbers)
            r"\[(\d+)\]",
        ];

        let regexes: Vec<Regex> = patterns.iter()
            .filter_map(|pattern| Regex::new(pattern).ok())
            .collect();

        let lines: Vec<&str> = content.lines().collect();
        let mut current_time = 0.0;
        let average_read_speed = 150.0; // words per minute

        for (index, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut segment_start_time = current_time;
            let mut segment_end_time: Option<f64> = None;
            let mut text = line.to_string();
            let mut found_timestamp = false;

            // First, try range timestamp formats [START-END]
            if let Some(captures) = range_hh_mm_ss.captures(line) {
                // [HH:MM:SS-HH:MM:SS] format
                let sh: f64 = captures[1].parse().unwrap_or(0.0);
                let sm: f64 = captures[2].parse().unwrap_or(0.0);
                let ss: f64 = captures[3].parse().unwrap_or(0.0);
                let eh: f64 = captures[4].parse().unwrap_or(0.0);
                let em: f64 = captures[5].parse().unwrap_or(0.0);
                let es: f64 = captures[6].parse().unwrap_or(0.0);

                segment_start_time = sh * 3600.0 + sm * 60.0 + ss;
                segment_end_time = Some(eh * 3600.0 + em * 60.0 + es);
                text = range_hh_mm_ss.replace(&text, "").trim().to_string();
                current_time = segment_start_time;
                found_timestamp = true;
            } else if let Some(captures) = range_mm_ss.captures(line) {
                // [MM:SS-MM:SS] format
                let sm: f64 = captures[1].parse().unwrap_or(0.0);
                let ss: f64 = captures[2].parse().unwrap_or(0.0);
                let em: f64 = captures[3].parse().unwrap_or(0.0);
                let es: f64 = captures[4].parse().unwrap_or(0.0);

                segment_start_time = sm * 60.0 + ss;
                segment_end_time = Some(em * 60.0 + es);
                text = range_mm_ss.replace(&text, "").trim().to_string();
                current_time = segment_start_time;
                found_timestamp = true;
            }

            // If no range format matched, try single-timestamp patterns
            if !found_timestamp {
                for regex in &regexes {
                    if let Some(captures) = regex.captures(line) {
                        let parsed_time = match captures.len() {
                            2 => {
                                // Single number (seconds or MM:SS without hours)
                                if let Ok(seconds) = captures.get(1).unwrap().as_str().parse::<f64>() {
                                    if seconds < 3600.0 {
                                        seconds
                                    } else {
                                        current_time
                                    }
                                } else {
                                    current_time
                                }
                            },
                            3 => {
                                // MM:SS format
                                let minutes: f64 = captures.get(1).unwrap().as_str().parse().unwrap_or(0.0);
                                let seconds: f64 = captures.get(2).unwrap().as_str().parse().unwrap_or(0.0);
                                minutes * 60.0 + seconds
                            },
                            4 => {
                                // HH:MM:SS format
                                let hours: f64 = captures.get(1).unwrap().as_str().parse().unwrap_or(0.0);
                                let minutes: f64 = captures.get(2).unwrap().as_str().parse().unwrap_or(0.0);
                                let seconds: f64 = captures.get(3).unwrap().as_str().parse().unwrap_or(0.0);
                                hours * 3600.0 + minutes * 60.0 + seconds
                            },
                            5 => {
                                // HH:MM:SS.mmm format with milliseconds
                                let hours: f64 = captures.get(1).unwrap().as_str().parse().unwrap_or(0.0);
                                let minutes: f64 = captures.get(2).unwrap().as_str().parse().unwrap_or(0.0);
                                let seconds: f64 = captures.get(3).unwrap().as_str().parse().unwrap_or(0.0);
                                let millis: f64 = captures.get(4)
                                    .map(|m| m.as_str().parse().unwrap_or(0.0))
                                    .unwrap_or(0.0) / 1000.0;
                                hours * 3600.0 + minutes * 60.0 + seconds + millis
                            },
                            _ => current_time
                        };

                        if parsed_time >= 0.0 {
                            segment_start_time = parsed_time;
                            current_time = segment_start_time;
                            text = regex.replace(&text, "").trim().to_string();
                            found_timestamp = true;
                            break;
                        }
                    }
                }
            }

            // Clean up text further - remove speaker names in format "Name:" at beginning
            text = text.trim_start_matches(':').trim().to_string();
            if text.ends_with(':') && text.split_whitespace().count() == 1 {
                // If text is just "Name:", skip this line
                continue;
            }

            if !text.is_empty() {
                // Use actual end_time from range format, or estimate from word count
                let word_count = text.split_whitespace().count();
                let estimated_duration = (word_count as f64 / average_read_speed) * 60.0;

                let end_time = if let Some(et) = segment_end_time {
                    // Range format provided an explicit end time
                    Some(et)
                } else if found_timestamp {
                    Some(segment_start_time + estimated_duration.max(1.0))
                } else {
                    current_time += estimated_duration.max(1.0);
                    Some(current_time)
                };

                segments.push(TranscriptionSegment {
                    start_time: segment_start_time,
                    end_time,
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

/// Try UTF-8 first; if invalid, fall back to Windows-1251 (common for Russian text files).
fn read_text_with_encoding(bytes: &[u8]) -> String {
    // Strip UTF-8 BOM if present
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);

    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (decoded, _, had_errors) = WINDOWS_1251.decode(bytes);
            if had_errors {
                // Last resort: lossy UTF-8
                String::from_utf8_lossy(bytes).to_string()
            } else {
                decoded.to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_range_mm_ss_format() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        let content = "[00:00-01:06] Спикер: Привет, это тестовая строка.\n\
                        [01:06-02:27] Спикер: Вторая строка текста.\n\
                        [02:27-03:55] Спикер: Третья строка.\n";
        let segments = merger.parse_txt(content, "test.txt").unwrap();

        assert_eq!(segments.len(), 3);

        // First segment: 0:00 to 1:06
        assert!((segments[0].start_time - 0.0).abs() < 0.01);
        assert!((segments[0].end_time.unwrap() - 66.0).abs() < 0.01);

        // Second segment: 1:06 to 2:27
        assert!((segments[1].start_time - 66.0).abs() < 0.01);
        assert!((segments[1].end_time.unwrap() - 147.0).abs() < 0.01);

        // Third segment: 2:27 to 3:55
        assert!((segments[2].start_time - 147.0).abs() < 0.01);
        assert!((segments[2].end_time.unwrap() - 235.0).abs() < 0.01);

        // Verify timestamps are removed from text
        assert!(!segments[0].text.contains("[00:00-01:06]"));
        assert!(!segments[0].text.contains(":00-01:06"));
    }

    #[test]
    fn test_parse_range_hh_mm_ss_format() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        let content = "[0:00:00-0:01:06] First line.\n\
                        [0:01:06-0:02:27] Second line.\n";
        let segments = merger.parse_txt(content, "test.txt").unwrap();

        assert_eq!(segments.len(), 2);
        assert!((segments[0].start_time - 0.0).abs() < 0.01);
        assert!((segments[0].end_time.unwrap() - 66.0).abs() < 0.01);
        assert!((segments[1].start_time - 66.0).abs() < 0.01);
        assert!((segments[1].end_time.unwrap() - 147.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_simple_mm_ss_format() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        let content = "[01:30] Some text here.\n\
                        [02:45] More text here.\n";
        let segments = merger.parse_txt(content, "test.txt").unwrap();

        assert_eq!(segments.len(), 2);
        assert!((segments[0].start_time - 90.0).abs() < 0.01);
        assert!((segments[1].start_time - 165.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_merge_two_files_with_range_timestamps() {
        let options = MergeOptions {
            output_format: FileFormat::Txt,
            time_offset_seconds: 0.0,
            remove_timestamps: false,
            add_file_markers: false,
        };
        let merger = TranscriptionMerger::new(options);

        // Simulate two files by parsing content directly
        let content1 = "[00:00-01:00] File one, segment one.\n\
                         [01:00-02:00] File one, segment two.\n";
        let content2 = "[00:00-01:30] File two, segment one.\n\
                         [01:30-03:00] File two, segment two.\n";

        let segments1 = merger.parse_txt(content1, "file1.txt").unwrap();
        let segments2 = merger.parse_txt(content2, "file2.txt").unwrap();

        // File 1: last segment ends at 120s (2:00)
        assert!((segments1.last().unwrap().end_time.unwrap() - 120.0).abs() < 0.01);
        // File 2: segments start at 0, last ends at 180s (3:00)
        assert!((segments2[0].start_time - 0.0).abs() < 0.01);
        assert!((segments2.last().unwrap().end_time.unwrap() - 180.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_real_file_format() {
        // Matches actual format from testing/ folder files
        let merger = TranscriptionMerger::new(MergeOptions::default());
        let content = "\
[00:00-01:06] Ведущий (модератор): Да, да, так мы начинаем же, если вы позволите же.\n\
[01:06-02:27] Алён: Привет. Я, наверное, я скажу те вещи.\n\
[02:27-03:55] Алён: Что стоит объяснить, вопрос к декану.\n";

        let segments = merger.parse_txt(content, "Транскрипция 1.txt").unwrap();

        assert_eq!(segments.len(), 3);

        // First segment starts at 0:00, ends at 1:06 (66s)
        assert!((segments[0].start_time - 0.0).abs() < 0.01);
        assert!((segments[0].end_time.unwrap() - 66.0).abs() < 0.01);

        // Text should NOT contain the timestamp bracket
        assert!(!segments[0].text.contains("[00:00-01:06]"));
        // Text should contain the speaker and content
        assert!(segments[0].text.contains("Ведущий"));

        // Second segment: 1:06 (66s) to 2:27 (147s)
        assert!((segments[1].start_time - 66.0).abs() < 0.01);
        assert!((segments[1].end_time.unwrap() - 147.0).abs() < 0.01);

        // Third segment: 2:27 (147s) to 3:55 (235s)
        assert!((segments[2].start_time - 147.0).abs() < 0.01);
        assert!((segments[2].end_time.unwrap() - 235.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_file_with_trailing_empty_lines() {
        // Real files have empty lines at the end
        let merger = TranscriptionMerger::new(MergeOptions::default());
        let content = "[00:00-01:00] First line.\n[01:00-02:00] Second line.\n\n\n\n";
        let segments = merger.parse_txt(content, "test.txt").unwrap();

        assert_eq!(segments.len(), 2);
        assert!((segments[1].end_time.unwrap() - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_srt_with_crlf() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        // Simulate Windows-style CRLF line endings
        let content = "1\r\n00:00:00,000 --> 00:00:05,000\r\nFirst subtitle.\r\n\r\n2\r\n00:00:05,000 --> 00:00:10,000\r\nSecond subtitle.\r\n\r\n";
        let segments = merger.parse_srt(content, "test.srt").unwrap();

        assert_eq!(segments.len(), 2);
        assert!((segments[0].start_time - 0.0).abs() < 0.01);
        assert!((segments[0].end_time.unwrap() - 5.0).abs() < 0.01);
        assert_eq!(segments[0].text, "First subtitle.");
        assert!((segments[1].start_time - 5.0).abs() < 0.01);
        assert!((segments[1].end_time.unwrap() - 10.0).abs() < 0.01);
        assert_eq!(segments[1].text, "Second subtitle.");
    }

    #[test]
    fn test_looks_like_srt_with_crlf() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        let content = "1\r\n00:00:00,000 --> 00:00:05,000\r\nSome text\r\n";
        assert!(merger.looks_like_srt(content));
    }

    #[test]
    fn test_txt_plain_numbers_not_treated_as_timestamps() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        // Text with plain numbers (years, amounts, list items) — should NOT be stripped
        let content = "В 2024 году произошло много событий.\n\
                        Сумма составила 15000 рублей.\n\
                        3 основных пункта были рассмотрены.\n";
        let segments = merger.parse_txt(content, "test.txt").unwrap();

        // All numbers should be preserved in the text
        assert!(segments.iter().any(|s| s.text.contains("2024")));
        assert!(segments.iter().any(|s| s.text.contains("15000")));
        assert!(segments.iter().any(|s| s.text.contains("3")));
    }

    #[test]
    fn test_txt_bracketed_seconds_still_work() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        // Bracketed numbers should still be treated as timestamps
        let content = "[120] Some text at two minutes.\n\
                        [300] Some text at five minutes.\n";
        let segments = merger.parse_txt(content, "test.txt").unwrap();

        assert_eq!(segments.len(), 2);
        assert!((segments[0].start_time - 120.0).abs() < 0.01);
        assert!((segments[1].start_time - 300.0).abs() < 0.01);
        // Timestamp should be removed from text
        assert!(!segments[0].text.contains("[120]"));
    }

    #[tokio::test]
    async fn test_merge_real_files_txt_format() {
        let test_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("testing");
        if !test_dir.exists() {
            eprintln!("Skipping: testing/ directory not found");
            return;
        }

        let mut files: Vec<String> = Vec::new();
        for i in 1..=4 {
            let path = test_dir.join(format!("Транскрипция {}.txt", i));
            if path.exists() {
                files.push(path.to_string_lossy().to_string());
            }
        }
        assert!(!files.is_empty(), "No test files found in testing/");

        let options = MergeOptions {
            output_format: FileFormat::Txt,
            time_offset_seconds: 0.0,
            remove_timestamps: false,
            add_file_markers: true,
        };
        let mut merger = TranscriptionMerger::new(options);
        merger.add_files(files.clone()).await.expect("Failed to add files");

        assert!(merger.get_file_count() > 0, "No files loaded");
        assert!(merger.get_total_segments() > 0, "No segments parsed");

        let result = merger.merge().await.expect("Merge failed");
        assert!(!result.is_empty(), "Merged output is empty");
        // TXT format should have timestamp brackets
        assert!(result.contains("["), "TXT output should contain timestamp brackets");
        println!("TXT merge: {} files, {} segments, {} chars output",
            merger.get_file_count(), merger.get_total_segments(), result.len());
    }

    #[tokio::test]
    async fn test_merge_real_files_srt_format() {
        let test_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("testing");
        if !test_dir.exists() {
            return;
        }

        let mut files: Vec<String> = Vec::new();
        for i in 1..=4 {
            let path = test_dir.join(format!("Транскрипция {}.txt", i));
            if path.exists() {
                files.push(path.to_string_lossy().to_string());
            }
        }
        if files.is_empty() { return; }

        let options = MergeOptions {
            output_format: FileFormat::Srt,
            time_offset_seconds: 0.0,
            remove_timestamps: false,
            add_file_markers: false,
        };
        let mut merger = TranscriptionMerger::new(options);
        merger.add_files(files).await.expect("Failed to add files");

        let result = merger.merge().await.expect("SRT merge failed");
        assert!(!result.is_empty(), "SRT output is empty");
        // SRT format should have --> arrows
        assert!(result.contains("-->"), "SRT output should contain --> timestamp arrows");
        println!("SRT merge: {} chars output", result.len());
    }

    #[tokio::test]
    async fn test_merge_real_files_md_format() {
        let test_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("testing");
        if !test_dir.exists() {
            return;
        }

        let mut files: Vec<String> = Vec::new();
        for i in 1..=4 {
            let path = test_dir.join(format!("Транскрипция {}.txt", i));
            if path.exists() {
                files.push(path.to_string_lossy().to_string());
            }
        }
        if files.is_empty() { return; }

        let options = MergeOptions {
            output_format: FileFormat::Markdown,
            time_offset_seconds: 0.0,
            remove_timestamps: false,
            add_file_markers: true,
        };
        let mut merger = TranscriptionMerger::new(options);
        merger.add_files(files).await.expect("Failed to add files");

        let result = merger.merge().await.expect("Markdown merge failed");
        assert!(!result.is_empty(), "MD output is empty");
        // Markdown should have headers
        assert!(result.contains("# Merged Transcription"), "MD output should have main header");
        assert!(result.contains("## "), "MD output should have file section headers");
        println!("MD merge: {} chars output", result.len());
    }

    #[test]
    fn test_extract_sequence_number() {
        let merger = TranscriptionMerger::new(MergeOptions::default());
        // Cyrillic filename with number
        assert_eq!(merger.extract_sequence_number("Транскрипция 1.txt"), Some(1));
        assert_eq!(merger.extract_sequence_number("Транскрипция 2.txt"), Some(2));
        assert_eq!(merger.extract_sequence_number("chunk_3.txt"), Some(3));
        assert_eq!(merger.extract_sequence_number("part-10.txt"), Some(10));
    }
}