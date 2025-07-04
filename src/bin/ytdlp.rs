use std::process::Command;
use std::path::PathBuf;
use std::io;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::prelude::IndexedRandom;
use walkdir::WalkDir;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Setup the librairies
    let libraries_dir = PathBuf::from("libs");
    let yt_dlp = libraries_dir.join("yt-dlp");

    // Select a random genre
    let genres = vec!["lofi", "pop"];
    let mut rng = thread_rng();
    let random_genre = genres.choose(&mut rng);

    // Set the name of the output file
    let output_dir = PathBuf::from("songs");
    let nb_song = WalkDir::new(&output_dir).into_iter().count() - 1;
    let output_file = output_dir.join("song".to_owned() + &nb_song.to_string() + ".%(ext)s");

    let search_base_prompt = "Best ".to_owned() + random_genre.unwrap() + " songs playlist";

    let url = "https://youtube.com/watch?v=kPa7bsKwL-c&list=PLDIoUOhQQPlXqz5QZ3dx-lh_p6RcPeKjv";

    let mut binding = Command::new(yt_dlp.to_str().unwrap());
    let status = binding.args([
        "--no-write-subs", 
        "-x", 
        "--audio-format", "mp3", 
        "--no-playlist", 
        "--add-metadata", 
        "-o", output_file.to_str().unwrap(), 
        url, 
    ]).status()?;

    Ok(())
}
