#![feature(str_split_remainder)]

use std::io::{self, BufReader};
use std::fs::File;
use std::collections::HashMap;
use rodio::{Decoder, OutputStream, Sink, source::EmptyCallback};
use id3::{Tag, Content};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

struct Player {
    m_song_infos: Vec<HashMap<String, String>>,
	end_of_song_signal: Arc<AtomicU32>,
}

fn add_signal_end_song(sink: &Sink, player: &mut Player) {
	let end_of_song_signal = player.end_of_song_signal.clone();
	sink.append(EmptyCallback::<i16>::new(Box::new(move || {
		end_of_song_signal.store(1, Ordering::Relaxed);
	})));
}

fn add_song_to_queue(sink: &Sink, path: &str, player: &mut Player) {
	let file = File::open(path).unwrap();
	let buffer = BufReader::new(file);
	let source = Decoder::new_mp3(buffer).unwrap();
	sink.append(source);
	add_signal_end_song(sink, player);
}

fn get_song_infos_from_file(path: &str) -> HashMap<String, String> {
	let file = File::open(path).unwrap();
	let tag = Tag::read_from2(&file).unwrap();
	let mut song_infos = HashMap::new();
	
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

	let (minutes, seconds) = get_audio_duration(path);
	song_infos.insert(String::from("duration"), minutes.to_string() + ":" + &seconds.to_string());

	song_infos
}

fn get_audio_duration(path: &str) -> (u32, u32) {
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
    let minutes = (duration_seconds / 60.0).floor() as u32;
    let seconds = (duration_seconds % 60.0).round() as u32;

    (minutes, seconds)
}

fn d_playing_infos(sink: &Sink, player: &mut Player) {
	if player.end_of_song_signal.load(Ordering::Relaxed) > 0 {
		player.m_song_infos.remove(0);
		player.end_of_song_signal.store(0, Ordering::Relaxed);
	}
    if sink.empty() {
        println!("No song is currently playing.");
    } else {
		if !player.m_song_infos.is_empty() {
	    	for (key, value) in player.m_song_infos.get(0).unwrap() {
	   			println!("{}: {}", key, value);
	    	}
		}
		println!("Position of song: {}s", sink.get_pos().as_secs());
		println!("Volume: {}", sink.volume());
		println!("Number of songs in queue: {}", sink.len() / 2);
    }
}

fn main() {
    let mut player = Player { m_song_infos: Vec::new(), end_of_song_signal: Arc::new(AtomicU32::new(0)) };
    let (_stream, handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&handle).unwrap();

    loop {
        println!("Enter a command !");
        let mut input = String::new();
        io::stdin()
                .read_line(&mut input)
                .unwrap();

		let mut args = input.trim_end().split(" ");
		let first_parameter = args.next().unwrap();
	
        match first_parameter {
	    	"infos" => {
				d_playing_infos(&sink, &mut player);
	    	},

            "pause" => {
                sink.pause();
            },

            "play" => {
                if !sink.is_paused() && args.remainder().is_some() {
					let song_name = args.next().unwrap();

					player.m_song_infos.push(get_song_infos_from_file(&song_name));
					add_song_to_queue(&sink, &song_name, &mut player);
				} else {
					sink.play();
				}
			},

			"skip" => {
				sink.skip_one();
				player.end_of_song_signal.store(1, Ordering::Relaxed);
			},

			"exit" => {
				break;
			},

        	_ => todo!(),
        }
    }
}
