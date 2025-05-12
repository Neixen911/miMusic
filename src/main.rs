#![feature(str_split_remainder)]

use std::io::{self, BufReader};
use std::fs::File;
use std::collections::HashMap;
use rodio::{Decoder, OutputStream, Sink};
use id3::{Tag, Content};
use symphonia::core::{formats::FormatOptions, meta::MetadataOptions, io::{MediaSourceStream, MediaSource}};
use symphonia::default::get_probe;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

struct Player {
    m_song_infos: HashMap<String, String>,
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

fn d_playing_infos(sink: &Sink, player: &Player) {
    if sink.empty() {
        println!("No song is currently playing.");
    } else {
		if !player.m_song_infos.is_empty() {
	    	for (key, value) in &player.m_song_infos {
	   			println!("{}: {}", key, value);
	    	}
		}
		println!("Position of song: {}s", sink.get_pos().as_secs());
		println!("Volume: {}", sink.volume());
		println!("Number of songs in queue: {}", sink.len() / 2);
    }
}

fn main() {
    let mut player = Player { m_song_infos: HashMap::new() };
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
		d_playing_infos(&sink, &player);
	    },

            "pause" => {
                sink.pause();
            },

            "play" => {
                if !sink.is_paused() && args.remainder().is_some() {
		    let song_name = args.next().unwrap();
                    let file = File::open(song_name).unwrap();
		    let tag = Tag::read_from2(&file).unwrap();
		    
		    for frame in tag.frames() {
			let id = frame.id();
			
			match frame.content() {
			    Content::Text(value) => {
				
				match id {
				    "TIT2" => {
					player.m_song_infos.insert(String::from("title"), value.to_string());
				    }

				    "TPE1" => {
					player.m_song_infos.insert(String::from("artist"), value.to_string());
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

        	let buffer = BufReader::new(file);
		    let source = Decoder::new_mp3(buffer).unwrap();
		    let (minutes, seconds) = get_audio_duration(song_name);
		    player.m_song_infos.insert(String::from("duration"), minutes.to_string() + ":" + &seconds.to_string());
		    sink.append(source);

		    let playlist_pos = Arc::new(AtomicU32::new(0));
		    let playlist_pos_clone = playlist_pos.clone();
		    sink.append(rodio::source::EmptyCallback::<i16>::new(Box::new(move || {
        		println!("empty callback is now running");
        		playlist_pos_clone.fetch_add(1, Ordering::Relaxed);
		    })));
		    println!("playlist_pos: {}", playlist_pos.load(Ordering::Relaxed));
                } else {
                    sink.play();
                }
            },

	    "skip" => {
			sink.skip_one();
	    },

		"exit" => {
			break;
		},

            _ => todo!(),
        }
    }
}
