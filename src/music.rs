use std::io::BufReader;
use std::fs::{self, File};
use std::collections::HashMap;
use rodio::{Decoder, Sink, source::EmptyCallback};
use id3::{Tag, Content};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct Player {
    pub m_song_infos: Vec<HashMap<String, String>>,
	pub end_of_song_signal: Arc<AtomicU32>,
}

// Return all the songs with their tags
pub fn get_all_songs() -> Vec<HashMap<String, String>> {
	let mut songs = Vec::new();
	let songs_path = fs::read_dir("songs").unwrap();

	for song_path in songs_path {
		let song_infos = get_song_infos_from_file(song_path.unwrap().path().to_str().unwrap());
		songs.push(song_infos);
	}

	songs
}

// Add signal to know when a song is ended
fn add_signal_end_song(sink: &Sink, player: &mut Player) {
	let end_of_song_signal = player.end_of_song_signal.clone();
	sink.append(EmptyCallback::<i16>::new(Box::new(move || {
		end_of_song_signal.store(1, Ordering::Relaxed);
	})));
}

// Add a song to the queue
pub fn add_song_to_queue(sink: &Sink, path: &str, player: &mut Player) {
	let file = File::open(path).unwrap();
	let buffer = BufReader::new(file);
	let source = Decoder::new_mp3(buffer).unwrap();
	sink.append(source);
	add_signal_end_song(sink, player);
}

// Return infos from song file
pub fn get_song_infos_from_file(path: &str) -> HashMap<String, String> {
	let file = File::open(path).unwrap();
	let tag = Tag::read_from2(&file).unwrap();
	let mut song_infos = HashMap::new();
	// Default datas
	song_infos.insert(String::from("path"), "songs/song.mp3".to_string());
	song_infos.insert(String::from("title"), "Unknown".to_string());
	song_infos.insert(String::from("artist"), "Unknown".to_string());
	song_infos.insert(String::from("duration"), "0".to_string());
	
	song_infos.insert(String::from("path"), path.to_string());
	
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

	let seconds = get_audio_duration(path);
	song_infos.insert(String::from("duration"), seconds.to_string());

	song_infos
}

// Return total duration of a song from a path (calcul from his frames and rate)
fn get_audio_duration(path: &str) -> u32 {
    let file = File::open(path).unwrap();
    let mss = MediaSourceStream::new(Box::new(file) as Box<dyn MediaSource>, Default::default());

    let probe = get_probe().format(
        &Default::default(),
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ).unwrap();

    let format = probe.format;
    let track = format.default_track().unwrap();
    let sample_rate = track.codec_params.sample_rate.unwrap();
    let duration_in_frames = track.codec_params.n_frames.unwrap();

    let duration_seconds = duration_in_frames as f64 / sample_rate as f64;

	duration_seconds as u32
}

// Return infos from the current playing song
pub fn get_current_song_info(sink: &Sink, player: &mut Player) -> Vec<String> {
	if player.end_of_song_signal.load(Ordering::Relaxed) > 0 {
		player.m_song_infos.remove(0);
		player.end_of_song_signal.store(0, Ordering::Relaxed);
	}

	let mut song_infos = Vec::new();
	if sink.empty() {
		song_infos.push("No song is currently playing.".to_string());
		song_infos.push("--".to_string());
		song_infos.push("0".to_string());
		song_infos.push("0".to_string());
	} else {
		if !player.m_song_infos.is_empty() {
			let actual_song = player.m_song_infos.get(0).unwrap();
			song_infos.push(actual_song.get("title").unwrap().to_string());
			song_infos.push(actual_song.get("artist").unwrap().to_string());
			song_infos.push(sink.get_pos().as_secs().to_string());
			song_infos.push(actual_song.get("duration").unwrap().to_string());
		}
	}

	song_infos
}
