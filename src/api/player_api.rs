use id3::{Content, Tag};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rodio::{Decoder, Sink, source::EmptyCallback};
use std::collections::HashMap;
use std::fs::{File, read_to_string};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;

use crate::settings;

// TODO: Mettre dans un fichier de conf cette valeur car dupliquer dans download_api
const OUTPUT_FILE_FORMAT: &str = "mp3";

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
	pub name_loop: Loop,
	pub songs_loop: Vec<HashMap<String, String>>,
}

impl Player {
	// Instantiate a Player
	pub fn new(sink: Sink) -> Self {
		Self {
			sink: sink,
            songs_queue: Vec::new(),
			previous_songs_queue: Vec::new(),
            end_of_song_signal: Arc::new(AtomicU32::new(0)),
			name_loop: Loop::None,
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

	// Return if Sink is paused or not
	pub fn is_paused(&mut self) -> bool {
		self.sink.is_paused()
	}

	// Set volume of sink
	pub fn set_volume(&mut self, new_volume: f32) {
		if new_volume <= 2.0 {
			self.sink.set_volume(new_volume);
			settings::update_settings("Volume".to_string(), new_volume);
		}
	}

	// Return volume of sink
	pub fn get_volume(&mut self) -> f32 {
		self.sink.volume()
	}

	// Setting to the next loop value
	pub fn next_songs_loop(&mut self) {
		match self.name_loop {
            Loop::None => {
                self.name_loop = Loop::Song;
            },
            Loop::Song => {
                self.name_loop = Loop::Queue;
            },
            Loop::Queue => {
                self.name_loop = Loop::None;
            }
        }
		self.set_loop();
	}

	// No loop / Loop actual song / Loop the queue
	pub fn set_loop(&mut self) {
		match self.name_loop {
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
		if !self.songs_queue.is_empty() {
			let playing_song = self.songs_queue.remove(0);
			let mut rng = thread_rng();
			self.songs_queue.shuffle(&mut rng);
			self.songs_queue.insert(0, playing_song);
		}
	}

	// Add a song to the queue
	pub fn add_song_to_queue(&mut self, path: &str) {
		let song = get_song_infos_from_file(path);
		self.songs_queue.push(song);
	}

	// Go to the previous / next song
	pub fn skip_one(&mut self, skip_direction: u32) {
		self.sink.skip_one();
		self.end_of_song_signal.store(skip_direction, Ordering::Relaxed);
	}

	// Return if Sink is empty or not
	pub fn empty(&mut self) -> bool {
		self.sink.empty()
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
				song_infos.push(self.sink.get_pos().as_secs().to_string()); // marche pas !?
				song_infos.push(actual_song.get("duration").expect("Unable to get duration !").to_string());
			}
		}
		song_infos.push(format!("Volume: {}%", (self.get_volume() * 100.0).round()));
		song_infos.push(self.get_loop());

		song_infos
	}

	// Update datas about song
	pub fn update_datas(&mut self) {
		// Remove song from the queue and change signal
		if self.end_of_song_signal.load(Ordering::Relaxed) > 0 {
			// Check if loop song is activated
			if self.songs_loop.len() == 1 {
				self.add_song_to_queue(&self.songs_loop[0].get("path").expect("Can't retrieve path of the song file !").to_owned());
			}
			// Determine the listening direction
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

		// Check if loop queue is activated
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
}

// Return total duration of a song from a path (calcul from his frames and rate)
pub fn get_audio_duration(path: &str) -> u32 {
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
pub fn get_song_infos_from_file(path: &str) -> HashMap<String, String> {
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

			let seconds = get_audio_duration(path);
			song_infos.insert(String::from("duration"), seconds.to_string());

			let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
			let playlists: Vec<settings::Playlist> = serde_json::from_str(&playlists_content)
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

// Convert seconds to minutes/seconds
pub fn seconds_to_minsec(seconds: f64) -> (u32, u32) {
    let min = (seconds / 60.0).floor() as u32;
    let sec = (seconds % 60.0).round() as u32;

    (min, sec)
}
