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

// Add signal to know when a song is ended
fn add_signal_end_song(sink: &Sink, player: &mut Player) {
	let end_of_song_signal = player.end_of_song_signal.clone();
	sink.append(EmptyCallback::<i16>::new(Box::new(move || {
		end_of_song_signal.store(1, Ordering::Relaxed);
	})));
}

// Add a song to the queue
pub fn add_song_to_queue(sink: &Sink, path: &str, player: &mut Player) {
	let file = File::open(path).expect("Unable to open file !");
	let buffer = BufReader::new(file);
	let source = Decoder::new_mp3(buffer).expect("Unable to make a MP3 Decoder !");
	sink.append(source);
	add_signal_end_song(sink, player);
}

// Download song(s) from a unique URL
fn download_songs_from(url: &str) {
    let libraries_dir = PathBuf::from("libs");
    let yt_dlp = libraries_dir.join("yt-dlp");

    // Fetching song(s) URL(S)
    let mut binding = Command::new(yt_dlp.to_str().expect("Unable to convert to str"));
    let status = binding.args([
        "--skip-download", 
        "--no-playlist", 
        "--print", "%(webpage_url)s", 
        url, 
    ]).output().expect("Failed to fetching song(s) url(s) !");

    let json_data = String::from_utf8_lossy(&status.stdout);
    let urls: Vec<String> = json_data
        .lines()
        .map(|line| line.trim().to_string())
        .collect();

    // Download song(s)
    let output_dir = PathBuf::from("songs");
    let mut id_song = WalkDir::new(&output_dir).into_iter().count() - 1;
    for song_url in urls {
        let filename = output_dir.join("song".to_owned() + &id_song.to_string() + ".%(ext)s");
        let mut binding = Command::new(yt_dlp.to_str().expect("Unable to convert to str"));
        let _status = binding.args([
            "--no-write-subs", 
            "-x", 
            "--audio-format", "mp3", 
            "--add-metadata", 
            "-o", filename.to_str().expect("Unable to convert to str"), 
            &song_url, 
        ]).output().expect("Failed to downloading song !");
        id_song = id_song + 1;
    }
}

// Return all the songs with their tags
pub fn get_all_songs() -> Vec<HashMap<String, String>> {
	let mut songs = Vec::new();
	let songs_path = fs::read_dir("songs").expect("Unable to find songs folder !");

	for song_path in songs_path {
		let song_infos = get_song_infos_from_file(song_path.expect("Songs folder is empty !").path().to_str().expect("Unable to convert to str"));
		songs.push(song_infos);
	}

	songs
}

// Return total duration of a song from a path (calcul from his frames and rate)
fn get_audio_duration(path: &str) -> u32 {
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
			let actual_song = player.m_song_infos.get(0).expect("Unable to get the actual song !");
			song_infos.push(actual_song.get("title").expect("Unable to get title !").to_string());
			song_infos.push(actual_song.get("artist").expect("Unable to get artist !").to_string());
			song_infos.push(sink.get_pos().as_secs().to_string());
			song_infos.push(actual_song.get("duration").expect("Unable to get duration !").to_string());
		}
	}

	song_infos
}

// Return infos from song file
pub fn get_song_infos_from_file(path: &str) -> HashMap<String, String> {
	let file = File::open(path).expect("Unable to open file !");
	let tag = Tag::read_from2(&file).expect("Unable to get tags from file !");
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
