#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod audio;
mod merger;
mod ffmpeg;

use commands::*;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_file_info,
            start_audio_processing,
            merge_transcriptions,
            export_merged_transcription,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}