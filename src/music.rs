use archive::{ArchiveExtractor, ArchiveFormat};
use id3::{Content, Frame as Id3Frame, Tag, TagLike, Version};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rodio::{Decoder, Sink, source::EmptyCallback};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, read_to_string, read_dir, remove_file, rename};
use std::io::{BufWriter, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;
use tokio::sync::watch::Sender;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::PermissionsExt;

const OUTPUT_FILE_FORMAT: &str = "mp3";
const OS: &str = std::env::consts::OS;
const ARCH: &str = std::env::consts::ARCH;

#[derive(Copy, Clone)]
pub enum Loop {
	None,
	Song,
	Queue
}

pub struct Player {
	pub sink: Sink,
    pub songs_queue: Vec<HashMap<String, String>>,
	pub previous_songs_queue: Vec<HashMap<String, String>>,
	pub end_of_song_signal: Arc<AtomicU32>,
	pub songs_loop: Vec<HashMap<String, String>>,
}

#[derive(Deserialize, Serialize)]
pub struct Playlist {
	pub playlist_name: String,
    pub songs_list: Vec<String>,
}

impl Player {
	// Instantiate a Player
	pub fn new(sink: Sink) -> Self {
		Self {
			sink: sink,
            songs_queue: Vec::new(),
			previous_songs_queue: Vec::new(),
            end_of_song_signal: Arc::new(AtomicU32::new(0)),
			songs_loop: Vec::new(),
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

	// Set volume of sink
	pub fn set_volume(&mut self, new_volume: f32) {
		if new_volume <= 3.0 {
			self.sink.set_volume(new_volume);
		}
	}

	// Return volume of sink
	pub fn get_volume(&mut self) -> f32 {
		self.sink.volume()
	}

	// Return if Sink is paused or not
	pub fn is_paused(&mut self) -> bool {
		self.sink.is_paused()
	}

	// No loop / Loop actual song / Loop the queue
	pub fn set_loop(&mut self, loop_value: Loop) {
		match loop_value {
			Loop::None => {
				self.songs_loop = Vec::new();
			}
			Loop::Song => {
				self.songs_loop.push(self.songs_queue[0].clone());
			},
			Loop::Queue => {
				self.songs_loop = self.songs_queue.clone();
			}
		}
	}

	// Return loop value
	pub fn get_loop(&mut self) -> String {
		let loop_value: String;
		match self.songs_loop.len() {
			0		=> { loop_value = String::from(""); },
			1		=> { loop_value = String::from("Song loop"); },
			2..		=> { loop_value = String::from("Queue loop"); }
		}

		loop_value
	}

	// Shuffle the queue
	pub fn shuffle_queue(&mut self) {
		let playing_song = self.songs_queue.remove(0);
		let mut rng = thread_rng();
    	self.songs_queue.shuffle(&mut rng);
		self.songs_queue.insert(0, playing_song);
	}

	// Return if Sink is empty or not
	pub fn empty(&mut self) -> bool {
		self.sink.empty()
	}

	// Go to the previous / next song
	pub fn skip_one(&mut self, skip_direction: u32) {
		self.sink.skip_one();
		self.end_of_song_signal.store(skip_direction, Ordering::Relaxed);
	}

	// Add signal to know when a song is ended
	pub fn add_signal_end_song(&mut self) {
		let end_of_song_signal_cloned = self.end_of_song_signal.clone();
		self.sink.append(EmptyCallback::new(Box::new(move || {
			if end_of_song_signal_cloned.load(Ordering::Relaxed) == 0 {
				end_of_song_signal_cloned.store(1, Ordering::Relaxed);
			}
		})));
	}

	// Add a song to the queue
	pub fn add_song_to_queue(&mut self, path: &str) {
		let song = self.get_song_infos_from_file(path);
		self.songs_queue.push(song);
	}

	// Update datas about song
	pub fn update_datas(&mut self) {
		// Remove song from the queue and change signal
		if self.end_of_song_signal.load(Ordering::Relaxed) > 0 {
			// Loop song
			if self.songs_loop.len() == 1 {
				self.add_song_to_queue(&self.songs_loop[0].get("path").expect("Can't retrieve path of the song file !").to_owned());
			}
			// Listening way (next or previous)
			match self.end_of_song_signal.load(Ordering::Relaxed) {
				1 => {
					self.previous_songs_queue.push(self.songs_queue.remove(0));
				},
				2 => {
					if self.previous_songs_queue.len() >= 1 {
						self.songs_queue.insert(0, self.previous_songs_queue.remove(self.previous_songs_queue.len() - 1));
					}
				},
				_ => {}
			}
			self.end_of_song_signal.store(0, Ordering::Relaxed);
		}

		// Loop queue
		if self.empty() {
			if self.songs_queue.len() != 0 {
				let path_song_to_play = self.songs_queue[0].get("path").expect("Can't retrieve path of the song file !");
				let file = File::open(path_song_to_play).expect("Unable to open file !");
				let source = Decoder::new_mp3(file).expect("Unable to make a MP3 Decoder !");
				self.sink.append(source);
				self.add_signal_end_song();
			}
			for song_id in 0..self.songs_loop.len() {
				let path = self.songs_loop[song_id].get("path").expect("Can't retrieve path of the song file !").to_owned();
				self.add_song_to_queue(&path);
			}
		}
	}

	// Return infos from the current playing song
	pub fn get_current_song_info(&mut self) -> Vec<String> {
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
		song_infos.push(format!("Volume: {}%", (self.get_volume() * 100.0).round()));
		song_infos.push(self.get_loop());

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
				// A optimiser
				song_infos.insert(String::from("is_normalized"), "false".to_string());
				
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
								"TNOB" => {
									song_infos.insert(String::from("is_normalized"), value.to_string());
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

// Return extension of librairies files
pub fn get_extension() -> String {
	let ytdlp_ext: &str = match (OS, ARCH) {
		("windows", "x86_64") 		=> "_x86.exe",
		("windows", "aarch64") 		=> "_aarch64.exe",
		("linux", "x86_64") 		=> "_x86",
		("linux", "aarch64") 		=> "_linux_aarch64",
		_ => ""
	};

	ytdlp_ext.to_string()
}

// Return filename of librairies to download
pub fn get_download_filename() -> (String, String, String, String) {
	let ytdlp_suffix = get_extension();
	let ytdlp_url = format!("https://github.com/yt-dlp/yt-dlp/releases/download/2026.03.03/yt-dlp{}", ytdlp_suffix);

	let (ffmpeg_url, ffmpeg_suffix) = match (OS, ARCH) {
		("windows", _) 				=> ("https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.7z", ".7z"),
		("linux", "x86_64") 		=> ("https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz", ".tar.xz"),
		("linux", "aarch64") 		=> ("https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-arm64-static.tar.xz", ".tar.xz"),
		_ => ("", "")
	};

	(ytdlp_url.to_string(), ytdlp_suffix, ffmpeg_url.to_string(), ffmpeg_suffix.to_string())
}

// Unix version of downloading librairies
#[cfg(unix)]
pub async fn download_libs(libraries_dir: &PathBuf) {
	let (ytdlp_download_url, ytdlp_extension, ffmpeg_download_url, ffmpeg_extension) = get_download_filename();
	println!("{}", &ffmpeg_download_url);

	// Download ytdlp
	let yt_dlp = libraries_dir.join(format!("ytdlp{}", ytdlp_extension));
	let mut destination = File::create(&yt_dlp).expect("Can't create library file");
	let bytes = reqwest::get(ytdlp_download_url).await.expect("Can't download ytdlp binary !")
		.bytes().await.expect("Can't retrieve ytdlp binary content !");
	let _ = destination.write_all(&bytes);
	let mode = 0o755;
	let mut permissions = destination.metadata().expect("Can't retrieve ytdlp destination file metadata !").permissions();
	permissions.set_mode(mode);
	destination.set_permissions(permissions).expect("Can't set new permissions to ytdlp file !");

	// Download ffmpeg
	let ffmpeg_filename = libraries_dir.join(format!("ffmpeg{}", ffmpeg_extension));
	let mut ffmpeg_destination = File::create(&ffmpeg_filename).expect("Can't create library file");
	let ffmpeg_bytes = reqwest::get(ffmpeg_download_url).await.expect("Can't download ffmpeg compressed file !")
		.bytes().await.expect("Can't retrieve ffmpeg compressed file !");
	let _ = ffmpeg_destination.write_all(&ffmpeg_bytes);
	let decompress_format = match OS {
		"windows" 	=> ArchiveFormat::SevenZ,
		"linux"		=> ArchiveFormat::TarXz,
		_default	=> ArchiveFormat::TarXz
	};
	let files = ArchiveExtractor::new().extract(&fs::read(&ffmpeg_filename).unwrap(), decompress_format).unwrap();
	let mut ext = "";
	if "windows" == OS {
		ext = ".exe";
	}
	let ffmpeg = libraries_dir.join(format!("ffmpeg{}", ext));
	let mut ffmpeg_content = Vec::new();
	for file in files {
		if file.path.contains(&format!("ffmpeg{}", ext)) {
			println!("{}", file.path);
			ffmpeg_content = file.data;
		}
	}
	let mut ffmpeg_dest = File::create(&ffmpeg).expect("Can't create library file");
	let _ = ffmpeg_dest.write_all(&ffmpeg_content);
	let ffmpeg_mode = 0o755;
	let mut ffmpeg_permissions = ffmpeg_dest.metadata().expect("Can't retrieve ffmpeg destination file metadata !").permissions();
	ffmpeg_permissions.set_mode(ffmpeg_mode);
	ffmpeg_dest.set_permissions(ffmpeg_permissions).expect("Can't set new permissions to ffmpeg file !");
}

// Windows version of downloading librairies
#[cfg(windows)]
pub async fn download_libs(libraries_dir: &PathBuf) {
	let (ytdlp_download_url, ytdlp_extension, ffmpeg_download_url, ffmpeg_extension) = get_download_filename();
	println!("{}", &ffmpeg_download_url);

	// Download ytdlp
	let yt_dlp = libraries_dir.join(format!("ytdlp{}", ytdlp_extension));
	let mut destination = File::create(&yt_dlp).expect("Can't create library file");
	let bytes = reqwest::get(ytdlp_download_url).await.expect("Can't download ytdlp binary !")
		.bytes().await.expect("Can't retrieve ytdlp binary content !");
	let _ = destination.write_all(&bytes);
	let mut permissions = destination.metadata().expect("Can't retrieve ytdlp destination file metadata !").permissions();
	permissions.set_readonly(false);
	destination.set_permissions(permissions).expect("Can't set new permissions to ytdlp file !");

	// Download ffmpeg
	let ffmpeg_filename = libraries_dir.join(format!("ffmpeg{}", ffmpeg_extension));
	let mut ffmpeg_destination = File::create(&ffmpeg_filename).expect("Can't create library file");
	let ffmpeg_bytes = reqwest::get(ffmpeg_download_url).await.expect("Can't download ffmpeg compressed file !")
		.bytes().await.expect("Can't retrieve ffmpeg compressed file !");
	let _ = ffmpeg_destination.write_all(&ffmpeg_bytes);
	let decompress_format = match OS {
		"windows" 	=> ArchiveFormat::SevenZ,
		"linux"		=> ArchiveFormat::TarXz,
		_default	=> ArchiveFormat::TarXz
	};
	let files = ArchiveExtractor::new().extract(&fs::read(&ffmpeg_filename).unwrap(), decompress_format).unwrap();
	let mut ext = "";
	if "windows" == OS {
		ext = ".exe";
	}
	let ffmpeg = libraries_dir.join(format!("ffmpeg{}", ext));
	let mut ffmpeg_content = Vec::new();
	for file in files {
		if file.path.contains(&format!("ffmpeg{}", ext)) {
			println!("{}", file.path);
			ffmpeg_content = file.data;
		}
	}
	let mut ffmpeg_dest = File::create(&ffmpeg).expect("Can't create library file");
	let _ = ffmpeg_dest.write_all(&ffmpeg_content);
	let mut ffmpeg_permissions = ffmpeg_dest.metadata().expect("Can't retrieve ffmpeg destination file metadata !").permissions();
	ffmpeg_permissions.set_readonly(false);
	ffmpeg_dest.set_permissions(ffmpeg_permissions).expect("Can't set new permissions to ffmpeg file !");
}

// Download song from a unique URL
pub async fn download_song(sender: Sender<(u32, u32, f64)>, song_url: String) {
	let libraries_dir = PathBuf::from("libs");
	if !libraries_dir.is_empty() {
		let _ = download_libs(&libraries_dir).await;
	}

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
		let _ = sender.send((0, 0, -99.9));
		return;
	}

    let output_dir = PathBuf::from("songs");
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
		"--ffmpeg-location", ffmpeg.to_str().expect("Unable to convert to str"), 
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

// Mofidying metadata of the song
pub fn modifying_metadata(filepath: String, new_song_datas: &Vec<(String, String)>) {
	let file = File::open(&filepath).expect("Unable to open file !");
	let mut tag = Tag::read_from2(&file).expect("Unable to get tags from file !");

	for (name, content) in new_song_datas {
		match name.as_str() {
			"TIT2" => {
				tag.set_title(content.to_string());
			}
			"TPE1" => {
				tag.set_artist(content.to_string());
			}
			"TNOB" => {
				tag.add_frame(Id3Frame::text("TNOB", content.to_string()));
			}
			&_ => {
				println!("{}, {}", name, content);
				continue;
			}
		}
	}

	tag.write_to_path(&filepath, Version::Id3v24).expect("Can't write metadata to the file");
}

// Normalize songs which required to normalize
pub async fn normalize_songs(sender: Sender<(u32, u32, f64)>) {
	let libraries_dir = PathBuf::from("libs");
	let songs_dir = PathBuf::from("songs");
	let ffmpeg = libraries_dir.join(format!("ffmpeg-{ARCH}"));

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
}