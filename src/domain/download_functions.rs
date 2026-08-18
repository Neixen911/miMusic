// A MODIFIER
use crate::music::{
    add_or_remove_song_to_playlist,
    modifying_metadata
};

use archive::{ArchiveExtractor, ArchiveFormat};
use id3::{Content, Frame as Id3Frame, Tag, TagLike, Version};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, read_dir, remove_file, rename};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tokio::sync::watch::Sender;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const OUTPUT_FILE_FORMAT: &str = "mp3";
const OS: &str = std::env::consts::OS;
const ARCH: &str = std::env::consts::ARCH;

// Function to return the ffmpeg path
pub fn get_ffmpeg_path() -> PathBuf {
	let libraries_dir = PathBuf::from("libs");
	let mut path = libraries_dir.join("ffmpeg");

	for file in read_dir(&libraries_dir).expect("Can't iter over library folder !") {
		let filename = file.expect("Can't retrieve lib file !").path();
		if filename.to_str().expect("Can't convert PathBuf into str !").contains("ffmpeg") {
			path = filename;
		}
	}

	path
}

// Return filename of librairies to download
pub fn get_download_filename() -> (String, String, String, String, String) {
	let ytdlp_suffix = match (OS, ARCH) {
		("windows", _) 				=> "_x86.exe",
		("linux", "x86_64") 		=> "_linux",
		("linux", "aarch64") 		=> "_linux_aarch64",
		_ => ""
	};
	let ytdlp_url = format!("https://github.com/yt-dlp/yt-dlp/releases/download/2026.03.03/yt-dlp{}", ytdlp_suffix);

	let (ffmpeg_url, ffmpeg_suffix, ffmpeg_server_suffix) = match (OS, ARCH) {
		("windows", _) 				=> ("https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z", ".7z", ".exe"),
		("linux", "x86_64") 		=> ("https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz", ".tar.xz", ""),
		("linux", "aarch64") 		=> ("https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-arm64-static.tar.xz", ".tar.xz", ""),
		_ => ("", "", "")
	};

	(
		ytdlp_url.to_string(),
		ytdlp_suffix.to_string(),
		ffmpeg_url.to_string(),
		ffmpeg_suffix.to_string(),
		ffmpeg_server_suffix.to_string()
	)
}

// Downloading librairies
pub async fn download_libs(libraries_dir: &PathBuf) {
	let (ytdlp_download_url, ytdlp_extension, ffmpeg_download_url, ffmpeg_extension, ffmpeg_server_extension) = get_download_filename();

	// Download ytdlp
	let yt_dlp = libraries_dir.join(format!("ytdlp{}", ytdlp_extension));
	let mut destination = File::create(&yt_dlp).expect("Can't create library file");
	let bytes = reqwest::get(ytdlp_download_url).await.expect("Can't download ytdlp binary !")
		.bytes().await.expect("Can't retrieve ytdlp binary content !");
	let _ = destination.write_all(&bytes);
	let mut permissions = destination.metadata().expect("Can't retrieve ytdlp destination file metadata !").permissions();
	#[cfg(windows)]
	permissions.set_readonly(false);
	#[cfg(unix)]
	permissions.set_mode(0o755);
	destination.set_permissions(permissions).expect("Can't set new permissions to ytdlp file !");

	// Download ffmpeg
	let ffmpeg_archive_filename = libraries_dir.join(format!("ffmpeg{}", ffmpeg_extension));
	let mut ffmpeg_destination = File::create(&ffmpeg_archive_filename).expect("Can't create library file");
	let ffmpeg_bytes = reqwest::get(ffmpeg_download_url).await.expect("Can't download ffmpeg compressed file !")
		.bytes().await.expect("Can't retrieve ffmpeg compressed file !");
	let _ = ffmpeg_destination.write_all(&ffmpeg_bytes);
	let decompress_format = match OS {
		"windows" 	=> ArchiveFormat::SevenZ,
		"linux"		=> ArchiveFormat::TarXz,
		_default	=> ArchiveFormat::TarXz
	};
	let files = ArchiveExtractor::new().extract(&fs::read(&ffmpeg_archive_filename).unwrap(), decompress_format).unwrap();
	let ffmpeg_filename = format!("ffmpeg{}", ffmpeg_server_extension);
	let ffmpeg = libraries_dir.join(&ffmpeg_filename);
	let mut ffmpeg_content = Vec::new();
	for file in files {
		let filename: Vec<&str> = file.path.split('/').collect();
		if filename[filename.len() - 1] == &ffmpeg_filename {
			ffmpeg_content = file.data;
		}
	}
	let mut ffmpeg_dest = File::create(&ffmpeg).expect("Can't create library file");
	let _ = ffmpeg_dest.write_all(&ffmpeg_content);
	let mut ffmpeg_permissions = ffmpeg_dest.metadata().expect("Can't retrieve ffmpeg destination file metadata !").permissions();
	#[cfg(windows)]
	ffmpeg_permissions.set_readonly(false);
	#[cfg(unix)]
	ffmpeg_permissions.set_mode(0o755);
	ffmpeg_dest.set_permissions(ffmpeg_permissions).expect("Can't set new permissions to ffmpeg file !");

	let _ = remove_file(&ffmpeg_archive_filename);
}

// Download song from a unique URL
pub fn download_song(sender: Sender<(u32, u32, f64)>, song_url: String, selected_playlist: String) {
    tokio::spawn( async move {
        if let Ok(stream) = TcpStream::connect("8.8.4.4:53") {
            stream.shutdown(Shutdown::Both).expect("Can't shutdown stream check !");
        } else {
            let _ = sender.send((0, 0, -98.0));
            return;
        }
        if !fs::exists("libs").expect("Non authorized folder check !") {
            let _ = fs::create_dir("libs");
        }
        let libraries_dir = PathBuf::from("libs");
        if read_dir(&libraries_dir).expect("Can't iter over library folder !").next().is_none() {
            let _ = sender.send((0, 0, -2.0));
            let _ = download_libs(&libraries_dir).await;
        }

        let _ = sender.send((0, 0, -3.0));
        let mut yt_dlp = PathBuf::new();
        let mut ffmpeg = PathBuf::new();
        for filename in read_dir(&libraries_dir).expect("Can't iter over library folder !") {
            let path = filename.expect("Can't retrieve file !").path();
            let path_str = path.as_path().to_str().expect("Can't convert path to str !");
            if path_str.contains("ytdlp") {
                yt_dlp = path;
            } else if path_str.contains("ffmpeg") {
                ffmpeg = path;
            }
        }
        
        if yt_dlp.is_empty() || ffmpeg.is_empty() {
            let _ = sender.send((0, 0, -99.0));
            return;
        }

        let mut existing_song = Vec::new();
        for filename in read_dir(PathBuf::from("songs")).expect("Can't iter over songs folder !") {
            let path = filename.expect("Can't retrieve file !").path();
            let path_str = path.as_path().to_str().expect("Can't convert path to str !").to_owned();
            let parsed_path = path_str.split("/").into_iter().last().expect("Can't retrieve last part of file path !").to_owned();
            existing_song.push(parsed_path);
        }

        let output_dir = PathBuf::from("songs");
        let mut downloaded_songs = Vec::new();

        let filename = output_dir.join("%(id)s-tmp");
        let mut binding = Command::new(yt_dlp.to_str().expect("Unable to convert to str"));
        let mut status = binding.args([
            "--progress",
            "--newline",
            "--no-write-subs", 
            "--no-playlist", 
            "-x", 
            "--audio-format", OUTPUT_FILE_FORMAT, 
            "--add-metadata", 
            "--print", "after_move:id=%(id)s", 
            "--ffmpeg-location", ffmpeg.to_str().expect("Unable to convert to str"), 
            "-o", filename.to_str().expect("Unable to convert to str"), 
            &song_url, 
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Can't download song !");

        let stdout = status.stdout.take().expect("Can't retrieve stdout of yt-dlp downloading process !");
        let mut lines = BufReader::new(stdout).lines();

        let mut index: u32 = 0;
        let mut total: u32 = 0;
        let mut percent: f64 = 0.0;

        while let Some(line) = lines.next() {
            let line = line.expect("Can't retrieve a None value !");
            if line.contains("[download] Downloading item") {
                // A optimiser (surement)
                index = line.split_whitespace()
                    .nth(3).expect("Can't get outside of the array !")
                    .parse::<u32>().expect("Can't convert &str to u32 !");
                total = line.split_whitespace()
                    .nth(5).expect("Can't get outside of the array !")
                    .parse::<u32>().expect("Can't convert &str to u32 !");
            }
            if line.contains("%") {
                percent = line.split_whitespace()
                    .find(|w| w.ends_with('%'))
                    .and_then(|p| Some(p.trim_end_matches('%')))
                    .and_then(|m: &str| Some(m.parse::<f64>().expect("Can't convert &str to f64 !")))
                    .expect("Can't truncate correctly percent value !");
            }
            if line.contains("id=") {
                downloaded_songs.push(line.split("=").into_iter().last().expect("Can't retrieve only the id of the downloaded song !").to_owned());
            }

            let _ = sender.send((index, total, percent));
        }
        let _ = status.wait().expect("Can't download song !");
        let _ = sender.send((0, 0, -4.0));
        
        let mut is_already_downloaded = false;
        for downloaded_song in downloaded_songs {
            let parent_folder = PathBuf::from("songs");
            let filename_normalized = downloaded_song.as_str().to_owned() + "-tmp." + OUTPUT_FILE_FORMAT;
            let filename = downloaded_song.as_str().to_owned() + "." + OUTPUT_FILE_FORMAT;
            if existing_song.contains(&filename) {
                if is_already_downloaded == false { is_already_downloaded = true; };
                let _ = remove_file(parent_folder.join(&filename_normalized));
            } else {
                let _ = rename(
                    parent_folder.join(&filename_normalized),
                    parent_folder.join(&filename)
                );
            }

            let dowloaded_songs_to_selected_playlist = env::var("DOWNLOADED_SONGS_TO_SELECTED_PLAYLIST")
                .expect("The configuration value is not set !")
                .parse::<bool>();
            if dowloaded_songs_to_selected_playlist.is_ok() {
                if dowloaded_songs_to_selected_playlist.ok().expect("Can't retrieve string to bool values !") {
                    println!("Song add to selected playlist !");
                    add_or_remove_song_to_playlist(parent_folder.join(&filename).to_str().expect("Can't convert PathBuf to str !").to_string(), &selected_playlist);
                }
            }
        }

        if is_already_downloaded {
            let _ = sender.send((0, 0, -51.0));
        } else {
            let _ = sender.send((0, 0, -1.0));
        }
    });
}

// Normalize songs which required to normalize
pub fn normalize_songs(sender: Sender<(u32, u32, f64)>) {
    tokio::spawn( async move {
        let ffmpeg = get_ffmpeg_path();
        let songs_dir = PathBuf::from("songs");

        let mut songs_to_normalized = Vec::new();
        for song in read_dir(songs_dir).expect("Can't iterate over songs folder !") {
            let path = song.expect("Can't retrieve song file !").path();
            let path_str = path.to_str().expect("Can't convert to str !").to_owned();
        
            let mut is_normalized = "false".to_string();
            if path_str.contains(OUTPUT_FILE_FORMAT) {
                let file = File::open(&path_str).expect("Unable to open file !");
                if let Ok(tag) = Tag::read_from2(&file) {
                    for frame in tag.frames() {
                        let id = frame.id();
                        match frame.content() {
                            Content::Text(value) => {
                                match id {
                                    "TNOB" => {
                                        is_normalized = value.to_string();
                                    }
                                    _default => {
                                        continue;
                                    }
                                }
                            }
                            _content => {
                                continue;
                            }
                        }
                    }
                }
            }

            if is_normalized.contains(&"false".to_string()) {
                songs_to_normalized.push(path_str);
            }
        }

        let mut normalizing_index = 0;
        let normalizing_total: u32 = songs_to_normalized.len().try_into().expect("Can't convert into u32 !");
        let mut calculated_time = 0.0;
        for song_path in &songs_to_normalized {
            normalizing_index = normalizing_index + 1;
            let _ = sender.send((
                normalizing_index,
                normalizing_total,
                calculated_time
            ));

            let filename = song_path;
            let mut binding = Command::new(ffmpeg.to_str().expect("Unable to convert to str"));
            let status = binding.args([
                "-i", filename,
                "-af",
                "loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json",
                "-f",
                "null",
                "-"
            ])
            .output()
            .expect("Can't retrieve output of normalize informations !");

            let necessary_datas = ["input_i", "input_tp", "input_lra", "input_thresh", "target_offset"];
            let json_data = String::from_utf8_lossy(&status.stderr);
            let mut iterate_data = json_data.split("\n");
            let mut datas = HashMap::new();

            while let Some(line) = iterate_data.next() {
                for required in necessary_datas {
                    if line.contains(required) {
                        let number: f64 = line
                            .split('"')
                            .filter_map(|s| s.parse::<f64>().ok())
                            .next()
                            .expect("Can't retrieve a valid number !");
                        datas.insert(required, number);
                        break;
                    }
                }
            }

            let filename_normalized = filename.replace(
                ".",
                "-tmp."
            );
            
            let concatenated_audio_params = format!(
                "loudnorm=I=-16:TP=-1.5:LRA=11:measured_I={:?}:measured_TP={:?}:measured_LRA={:?}:measured_thresh={:?}:offset={:?}:linear=true",
                datas.get(necessary_datas[0]).expect("Can't retrieve input_i value !"),
                datas.get(necessary_datas[1]).expect("Can't retrieve input_tp value !"),
                datas.get(necessary_datas[2]).expect("Can't retrieve input_lra value !"),
                datas.get(necessary_datas[3]).expect("Can't retrieve input_thresh value !"),
                datas.get(necessary_datas[4]).expect("Can't retrieve target_offset value !")
            );
            
            let mut second_binding = Command::new(ffmpeg.to_str().expect("Unable to convert to str"));
            let _second_status = second_binding.args([
                "-i", filename,
                "-af",
                &concatenated_audio_params,
                "-ar",
                "48000",
                &filename_normalized
            ])
            .output()
            .expect("Can't retrieve output of normalize informations !");

            let _ = rename(filename_normalized, filename);

            let mut normalized = Vec::new();
            normalized.push(("TNOB".to_string(), "true".to_string()));
            modifying_metadata(filename.to_string(), &normalized);

            // A optimiser: récupérer le temps de vidéo déjà normalisé et calculer le temps restant
            calculated_time = (100 * normalizing_index / normalizing_total) as f64
        }

        let _ = sender.send((
            normalizing_total,
            normalizing_total,
            100.0
        ));
    });
}