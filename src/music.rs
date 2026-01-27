use id3::{Content, Tag};
use rodio::{Decoder, Sink, source::EmptyCallback};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, read_to_string, remove_file};
use std::io::{BufWriter, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;
use tokio::sync::watch::Sender;

const OUTPUT_FILE_FORMAT: &str = "mp3";
const ARCH: &str = std::env::consts::ARCH;

pub struct Player {
	pub sink: Sink,
    pub songs_queue: Vec<HashMap<String, String>>,
	pub end_of_song_signal: Arc<AtomicU32>,
}

#[derive(Deserialize, Serialize)]
pub struct Playlist {
	pub playlist_name: String,
    pub songs_list: Vec<String>,
}

impl Player {
	// Instantiate a Player
	pub fn new(sink: Sink, songs_queue: Vec<HashMap<String, String>>, end_of_song_signal: Arc<AtomicU32>) -> Self {
		Self {
			sink,
			songs_queue,
			end_of_song_signal,
		}
	}

	// Play a song
	pub fn play(&mut self) {
		self.sink.play();
	}

	// Pause a song
	pub fn pause(&mut self) {
		self.sink.pause();
	}

	// Return if Sink is paused or not
	pub fn is_paused(&mut self) -> bool {
		self.sink.is_paused()
	}

	// Return if Sink is empty or not
	pub fn empty(&mut self) -> bool {
		self.sink.empty()
	}

	// Skip the current song
	pub fn skip_one(&mut self) {
		self.sink.skip_one();
		self.end_of_song_signal.store(1, Ordering::Relaxed);
	}

	// Add signal to know when a song is ended
	pub fn add_signal_end_song(&mut self) {
		let end_of_song_signal_cloned = self.end_of_song_signal.clone();
		self.sink.append(EmptyCallback::new(Box::new(move || {
			end_of_song_signal_cloned.store(1, Ordering::Relaxed);
		})));
	}

	// Add a song to the queue
	pub fn add_song_to_queue(&mut self, path: &str) {
		let file = File::open(path).expect("Unable to open file !");
		let source = Decoder::new_mp3(file).expect("Unable to make a MP3 Decoder !");
		self.sink.append(source);
		let song = self.get_song_infos_from_file(path);
		self.songs_queue.push(song);
		self.add_signal_end_song();
	}

	// Return infos from the current playing song
	pub fn get_current_song_info(&mut self) -> Vec<String> {
		if self.end_of_song_signal.load(Ordering::Relaxed) > 0 {
			self.songs_queue.remove(0);
			self.end_of_song_signal.store(0, Ordering::Relaxed);
		}

		let mut song_infos = Vec::new();
		if self.empty() {
			song_infos.push("No song is currently playing.".to_string());
			song_infos.push("--".to_string());
			song_infos.push("0".to_string());
			song_infos.push("0".to_string());
		} else {
			if !self.songs_queue.is_empty() {
				let actual_song = self.songs_queue.get(0).expect("Unable to get the actual song !");
				song_infos.push(actual_song.get("title").expect("Unable to get title !").to_string());
				song_infos.push(actual_song.get("artist").expect("Unable to get artist !").to_string());
				song_infos.push(self.sink.get_pos().as_secs().to_string());
				song_infos.push(actual_song.get("duration").expect("Unable to get duration !").to_string());
			}
		}

		song_infos
	}

	// Return total duration of a song from a path (calcul from his frames and rate)
	pub fn get_audio_duration(&mut self, path: &str) -> u32 {
		let file = File::open(path).expect("Unable to open file !");
		let mss = MediaSourceStream::new(Box::new(file) as Box<dyn MediaSource>, Default::default());

		let probe = get_probe().format(
			&Default::default(),
			mss,
			&FormatOptions::default(),
			&MetadataOptions::default(),
		).expect("Unable to get datas usefull for calculate audio duration !");

		let format = probe.format;
		let track = format.default_track().expect("Unable to get track !");
		let sample_rate = track.codec_params.sample_rate.expect("Unable to get sample_rate !");
		let duration_in_frames = track.codec_params.n_frames.expect("Unable to get duration_in_frames !");

		let duration_seconds = duration_in_frames as f64 / sample_rate as f64;

		duration_seconds as u32
	}

	// Return infos from song file
	pub fn get_song_infos_from_file(&mut self, path: &str) -> HashMap<String, String> {
		let mut song_infos = HashMap::new();
		let mut is_song = false;
		
		if path.contains(OUTPUT_FILE_FORMAT) {
			let file = File::open(path).expect("Unable to open file !");

			if let Ok(tag) = Tag::read_from2(&file) {	
				is_song = true;

				// Default datas
				song_infos.insert(String::from("path"), path.to_string());
				song_infos.insert(String::from("title"), "Unknown".to_string());
				song_infos.insert(String::from("artist"), "Unknown".to_string());
				song_infos.insert(String::from("duration"), "0".to_string());
				song_infos.insert(String::from("is_favorite"), "♡".to_string());
				
				for frame in tag.frames() {
					let id = frame.id();
				
					match frame.content() {
						Content::Text(value) => {
							match id {
								"TIT2" => {
									song_infos.insert(String::from("title"), value.to_string());
								}
								"TPE1" => {
									song_infos.insert(String::from("artist"), value.to_string());
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

				let seconds = self.get_audio_duration(path);
				song_infos.insert(String::from("duration"), seconds.to_string());

				let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
				let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
					.expect("Playlists JSON content is not well-formatted !");
				for playlist in playlists {
					if playlist.playlist_name == "Favorites" {
						if playlist.songs_list.contains(&path.to_string()) {
							song_infos.insert(String::from("is_favorite"), "♥".to_string());
						}
						break;
					}
				}
			}
		}
		song_infos.insert(String::from("is_song"), is_song.to_string());

		song_infos
	}

	pub fn set_favorites(&mut self, path: &str) {
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");
		for playlist in &mut playlists {
			if playlist.playlist_name == "Favorites" {
				if playlist.songs_list.contains(&path.to_string()) {
					// Delete song from 'Favorites' playlist
					let position = playlist.songs_list.iter().position(|n| n == &path.to_string()).expect("Can't get position of path into JSON file !");
					playlist.songs_list.swap_remove(position);
				} else {
					// Add song to 'Favorites' playlist
					playlist.songs_list.push(path.to_string());
				}

				let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
				let mut playlists_writer = BufWriter::new(playlists_file);
				let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
				let _ = playlists_writer.flush();
				break;
			}
		}
	}

	// Return all the songs with their tags from the active playlist
	pub fn get_all_songs_from_active_playlist(&mut self, playlist_name: &String) -> Vec<HashMap<String, String>> {
		let mut songs = Vec::new();
		let songs_path = if fs::exists("songs").expect("Non authorized folder check !") {
			fs::read_dir("songs").expect("Can't retrieve songs folder !")
		} else {
			let _ = fs::create_dir("songs");
			fs::read_dir("songs").expect("Can't retrieve songs folder !")
		};

		if !fs::exists("playlists.json").expect("Non authorized folder check !") {
			let mut new: Vec<Playlist> = Vec::new();
			new.push(Playlist { playlist_name: "All songs".to_string(), songs_list: Vec::new() });

			let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
			let mut playlists_writer = BufWriter::new(playlists_file);
			let _ = serde_json::to_writer(&mut playlists_writer, &new);
			let _ = playlists_writer.flush();
		}

		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");
		let is_playlist = playlists.iter().position(|playlist| playlist.playlist_name == *playlist_name);
		if is_playlist.is_some() {
			for playlist in playlists {
				if &playlist.playlist_name == playlist_name {
					for song_path in songs_path {
						let song_infos = self.get_song_infos_from_file(song_path.expect("Songs folder is empty !").path().to_str().expect("Unable to convert to str"));
						if song_infos.get("is_song").expect("Can't get is_song variable !") == "true" {
							if playlist_name == "All songs" || playlist.songs_list.contains(song_infos.get("path").expect("Can't get path variable !")) {
								songs.push(song_infos);
							}
						}
					}
					break;
				}
			}
		}

		songs
	}

	// Return all playlists datas
	pub fn get_all_playlists(&mut self) -> Vec<Playlist> {
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");

		playlists
	}

	// Add a new playlist
	pub fn add_playlist(&mut self) {
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");

		// Return the next index to set the playlist name
		let mut index = playlists.len();
		for i in 0..index {
			if !playlists.iter().any(|playlist| playlist.playlist_name == format!("Playlist {}", i)) {
				index = i;
				break
			}
		};

		let new = Playlist {
			playlist_name: "Playlist ".to_string() + index.to_string().as_str(),
			songs_list: Vec::new(),
		};
		playlists.push(new);
		let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
		let mut playlists_writer = BufWriter::new(playlists_file);
		let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
		let _ = playlists_writer.flush();
	}

	// Modify the selected playlist
	pub fn modify_playlist(&mut self, actual_playlist_position: usize, new_playlist_name: &String) {
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");

		let mut playlist: Playlist = playlists.remove(actual_playlist_position);
		playlist.playlist_name = new_playlist_name.to_string();
		playlists.insert(actual_playlist_position, playlist);

		let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
		let mut playlists_writer = BufWriter::new(playlists_file);
		let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
		let _ = playlists_writer.flush();
	}

	// Remove the selected playlist
	pub fn remove_playlist(&mut self, playlist_position_to_remove: usize) {
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");
		// Delete playlist
		playlists.remove(playlist_position_to_remove);
		let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
		let mut playlists_writer = BufWriter::new(playlists_file);
		let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
		let _ = playlists_writer.flush();
	}

	// Add OR Remove song from a playlist
	pub fn add_or_remove_song_to_playlist(&mut self, song_to_add: String, selected_playlist: String) {
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");
		for playlist in &mut playlists {
			if playlist.playlist_name == selected_playlist {
				if playlist.songs_list.contains(&song_to_add) {
					// Delete song from the playlist
					let position = playlist.songs_list.iter().position(|n| *n == song_to_add).expect("Can't get position of path into JSON file !");
					playlist.songs_list.swap_remove(position);
				} else {
					// Add song to the playlist
					playlist.songs_list.push(song_to_add);
				}

				let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
				let mut playlists_writer = BufWriter::new(playlists_file);
				let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
				let _ = playlists_writer.flush();
				break;
			}
		}
	}

	// Remove the selected song
	pub fn remove_song(&mut self, song_to_remove: String) {
		// Remove song from all playlists
		let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
		let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
			.expect("Playlists JSON content is not well-formatted !");
		for playlist in &mut playlists {
			if playlist.songs_list.contains(&song_to_remove) {
				let position = playlist.songs_list.iter().position(|n| *n == song_to_remove).expect("Can't get position of path into JSON file !");
				playlist.songs_list.swap_remove(position);
			}
		}

		let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
		let mut playlists_writer = BufWriter::new(playlists_file);
		let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
		let _ = playlists_writer.flush();

		// Remove song itself
		let _ = remove_file(&song_to_remove);
	}
}

// Download song from a unique URL
pub async fn download_song(sender: Sender<(u32, u32, f64)>, song_url: String) {
    let libraries_dir = PathBuf::from("libs");
    let yt_dlp = libraries_dir.join(format!("yt-dlp-{ARCH}"));
    let output_dir = PathBuf::from("songs");

	let ffmpeg_location = libraries_dir.join(format!("ffmpeg-{ARCH}"));
	let filename = output_dir.join("%(id)s");

	let mut binding = Command::new(yt_dlp.to_str().expect("Unable to convert to str"));
	let mut status = binding.args([
		"--progress",
		"--newline",
		"--no-write-subs", 
		"--no-playlist", 
		"-x", 
		"--audio-format", OUTPUT_FILE_FORMAT, 
		"--add-metadata", 
		"--ffmpeg-location", ffmpeg_location.to_str().expect("Unable to convert to str"), 
		"-o", filename.to_str().expect("Unable to convert to str"), 
		&song_url, 
	])
	.stdout(Stdio::piped())
	.stderr(Stdio::piped())
	.spawn()
	.expect("Can't download song !");

	let stdout = status.stdout.take().expect("Can't retrieve stdout of yt-dlp downlading process !");
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

		let _ = sender.send((index, total, percent));
    }
    let _ = status.wait().expect("Can't download song !");
	let _ = sender.send((0, 0, -1.0));
}
