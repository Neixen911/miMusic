#![feature(str_split_remainder)]

use std::io;
use std::io::BufReader;
use std::fs::File;
use std::collections::HashMap;
use rodio::{Decoder, OutputStream, Sink};
use id3::{Tag, Content};

struct Player {
    m_song_infos: HashMap<String, String>,
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
	println!("Nombre de musiques en attente: {}", sink.len());
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
            "pause" => {
                sink.pause();
                d_playing_infos(&sink, &player);
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
		    sink.append(Decoder::new_mp3(buffer).unwrap());
		    d_playing_infos(&sink, &player);
                } else {
                    sink.play();
		    d_playing_infos(&sink, &player);
                }
            },

            "exit" => {
                break;
            },

            _ => todo!(),
        }
    }
}
