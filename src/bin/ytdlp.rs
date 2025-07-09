use std::process::Command;
use std::path::PathBuf;
use std::io;
use std::fs::File;
use std::io::Write;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::prelude::IndexedRandom;
use walkdir::WalkDir;
use serde_json::Value;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Setup the librairies
    let libraries_dir = PathBuf::from("libs");
    let yt_dlp = libraries_dir.join("yt-dlp");
    let url = "https://www.youtube.com/playlist?list=PLOzDu-MXXLlg5384VEAWMzgXSdU8HFwbS";

    println!("Starting fetching songs from playlist ...");

    // Fetching songs URL from playlist
    let mut binding = Command::new(yt_dlp.to_str().unwrap());
    let status = binding.args([
        "--skip-download", 
        "--print", "%(webpage_url)s", 
        url, 
    ]).output().unwrap();

    println!("Done !");

    // String output to vec of urls
    let json_data = String::from_utf8_lossy(&status.stdout);
    let urls: Vec<String> = json_data
        .lines()
        .map(|line| line.trim().to_string())
        .collect();

    println!("Starting to download songs ...");

    // Download songs from playlist
    let output_dir = PathBuf::from("songs");
    let mut id_song = WalkDir::new(&output_dir).into_iter().count() - 1;
    for song_url in urls {
        let filename = output_dir.join("song".to_owned() + &id_song.to_string() + ".%(ext)s");
        let mut binding = Command::new(yt_dlp.to_str().unwrap());
        let status = binding.args([
            "--no-write-subs", 
            "-x", 
            "--audio-format", "mp3", 
            "--add-metadata", 
            "-o", filename.to_str().unwrap(), 
            &song_url, 
        ]).status()?;
        println!("Song {} downloaded !", song_url);
        id_song = id_song + 1;
    }

    println!("Done !");

    Ok(())
}
