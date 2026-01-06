use id3::{Content, Tag, TagLike, Version};
use regex::Regex;
use rodio::{Decoder, Sink, source::EmptyCallback};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, read_to_string};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::Command;
use std::str::Chars;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;
use walkdir::WalkDir;

const OUTPUT_FILE_FORMAT: &str = "mp3";
const BASE_3_MIN_DOWNLOADING_TIME: f64 = 30.0;
const MINUTE_SUPPLEMENTARY: f64 = 2.5;

const SONGS_FOLDER_PATH: PathBuf = 

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

		let _ = File::create("playlists.json").expect("Failed to create/open playlists.json");
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
}

// Retrieve data(s) song(s) from a unique URL
pub async fn retrieve_songs_datas_from(url: &str) -> (Vec<String>, f64) {
    let libraries_dir = PathBuf::from("libs");
    let yt_dlp = libraries_dir.join("yt-dlp");

    // Fetching song(s) data(S)
    let mut binding = Command::new(yt_dlp.to_str().expect("Unable to convert to str"));
    let status = binding.args([
        "--skip-download", 
        "--no-playlist", 
        "--print", "%(webpage_url)s %(duration)s", 
        url, 
    ]).output().expect("Failed to fetching song(s) data(s) !");

    let json_data = String::from_utf8_lossy(&status.stdout);
    let urls: Vec<String> = json_data
		.lines()
		.filter_map(|line| line.split_whitespace().nth(0))
		.map(|word| word.to_string())
		.collect();
	let durations: Vec<String> = json_data
		.lines()
		.filter_map(|line| line.split_whitespace().nth(1))
		.map(|word| word.to_string())
		.collect();
	let mut estimated_downloading_durations: f64 = 0.0;
	for duration in durations {
		let min_song: f64 = duration.parse::<f64>().expect("Can't convert String to f64 !") / 60.0;
		let mut estimated_downloading_duration: f64 = BASE_3_MIN_DOWNLOADING_TIME;
		if min_song > 3.0 {
			estimated_downloading_duration = estimated_downloading_duration + ((min_song - 3.0) * MINUTE_SUPPLEMENTARY).ceil();
		}
		estimated_downloading_durations = estimated_downloading_durations + estimated_downloading_duration;
	}

	(urls, estimated_downloading_durations)
}

// Download song from a unique URL
pub async fn download_song(song_url: String) {
    let libraries_dir = PathBuf::from("libs");
    let yt_dlp = libraries_dir.join("yt-dlp");
    let output_dir = PathBuf::from("songs");
	
    let id_song = WalkDir::new(&output_dir).into_iter().count() - 1;
	let filename = output_dir.join("song".to_owned() + &id_song.to_string() + "." + OUTPUT_FILE_FORMAT);

	let mut binding = Command::new(yt_dlp.to_str().expect("Unable to convert to str"));
	let _status = binding.args([
		"--quiet", 
		"--no-write-subs", 
		"-x", 
		"--audio-format", OUTPUT_FILE_FORMAT, 
		"--add-metadata", 
		"-o", filename.to_str().expect("Unable to convert to str"), 
		&song_url, 
	]).output().expect("Can't download song !");

	applying_metadata("songs/song".to_owned() + &id_song.to_string() + "." + OUTPUT_FILE_FORMAT);
}

// Applying new metadata to the file
fn applying_metadata(filename: String) {
	let file = File::open(&filename).expect("Unable to open file !");
	let mut tag = Tag::read_from2(&file).expect("Unable to get tags from file !");

	// Parsing tags
	let mut new_title = tag.title().expect("Can't get title !").to_string();
	let new_artist = tag.artist().expect("Can't get artist !").to_string();

	if new_title.contains(&new_artist) {
		new_title = new_title.replace(&new_artist, "");
	}

	// List of commons regex to remove
	let list_regex = [ r"\(.*\)", r"\[.*\]", r".*『", r"』.*", r".*「", r"」.*", r".*-", r" feat.*", r"ft.*", r"by.*" ];
	for regex in list_regex {
		let regex_to_remove = Regex::new(regex).expect("Can't create Regex !");
		new_title = regex_to_remove.replace_all(&new_title, "").to_string();
	}

	let mut position: u8 = 0;
	let tmp = new_title.clone();
	let new_title_iter = tmp.chars();

	for character in new_title_iter {
		if !character.is_alphabetic() && have_whitespace_after(&mut new_title.chars(), position) {
			new_title = new_title.replace(character, " ");
		}
		position = position + 1;
	}

	let spaces_regex = Regex::new(r" {2,}").expect("Can't create Regex !");
	new_title = spaces_regex.replace_all(&new_title, " ").to_string();

	// Setting tags
	tag.set_title(new_title.trim());
	tag.set_artist(new_artist.trim());

	tag.write_to_path(&filename, Version::Id3v24).expect("Can't write metadata to the file");
}

// Check if special character have whitespace it
fn have_whitespace_after(title: &mut Chars<'_>, position: u8) -> bool {
	let length: usize = title.clone().count() - 1;
	let next_character_position: usize = (position + 1).into();
	
	if next_character_position <= length {
		if title.nth(next_character_position).expect("Can't get next character !").is_whitespace() {
			return true;
		} else {
			return false;
		}
	} else {
		return false;
	}
}
