
use id3::{Content, Frame as Id3Frame, Tag, TagLike, Version};
use std::collections::HashMap;
use std::fs::{self, File, read_to_string, remove_file};
use std::io::{BufWriter, Write};

use crate::api::{self, Playlist};
use crate::settings;

// TODO: Mettre dans un fichier de conf cette valeur car dupliquer dans download_api
const OUTPUT_FILE_FORMAT: &str = "mp3";

// Retrieve only metadata that can be modified
pub fn get_modify_metadata(filepath: String) -> Vec<(String, String)> {
	let mut metadata = Vec::new();
	let song = get_all_metadata(filepath);

	let song_entitled = ["TIT2", "TPE1"];
	let mut song_value: &str;

	for entitled_name in song_entitled {
		match entitled_name {
			"TIT2" => {
				song_value = "title";
			}
			"TPE1" => {
				song_value = "artist";
			}
			_default => {
				continue;
			}
		}
		metadata.push((entitled_name.to_string(), song.get(song_value).expect("Can't retrieve a specific value of a song !").to_string()));
	}

	metadata
}

// Retrieve all metadata of the song
pub fn get_all_metadata(filepath: String) -> HashMap<String, String> {
	let mut song_infos = HashMap::new();
	let mut is_song = false;
	
	if filepath.contains(OUTPUT_FILE_FORMAT) {
		let file = File::open(&filepath).expect("Unable to open file !");

		if let Ok(tag) = Tag::read_from2(&file) {	
			is_song = true;

			// Default datas
			song_infos.insert(String::from("path"), filepath.to_string());
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

			let seconds = api::get_audio_duration(&filepath);
			song_infos.insert(String::from("duration"), seconds.to_string());

			let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
			let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
				.expect("Playlists JSON content is not well-formatted !");
			for playlist in playlists {
				if playlist.playlist_name == "Favorites" {
					if playlist.songs_list.contains(&filepath) {
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

// Modify metadata of the song
pub fn set_metadata(filepath: String, new_song_datas: &Vec<(String, String)>) {
	let file = File::open(&filepath).expect("Unable to open file !");
	let mut song = Tag::read_from2(&file).expect("Unable to get tags from file !");

	for (name, content) in new_song_datas {
		match name.as_str() {
			"TIT2" => {
				song.set_title(content.to_string());
			}
			"TPE1" => {
				song.set_artist(content.to_string());
			}
			"TNOB" => {
				song.add_frame(Id3Frame::text("TNOB", content.to_string()));
			}
			&_ => {
				println!("{}, {}", name, content);
				continue;
			}
		}
	}

	song.write_to_path(&filepath, Version::Id3v24).expect("Can't write metadata to the file");
}

// Remove the selected song
pub fn remove_song(song_to_remove: String) {
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
	let _ = serde_json::to_writer_pretty(&mut playlists_writer, &playlists);
	let _ = playlists_writer.flush();

	// Remove song itself
	let _ = remove_file(&song_to_remove);
}

// Return all the songs from the active playlist
pub fn get_all_songs(playlist_name: &String) -> Vec<HashMap<String, String>> {
	settings::verify_files();
	let mut songs = Vec::new();
	let songs_path = if fs::exists("songs").expect("Non authorized folder check !") {
		fs::read_dir("songs").expect("Can't retrieve songs folder !")
	} else {
		let _ = fs::create_dir("songs");
		fs::read_dir("songs").expect("Can't retrieve songs folder !")
	};

	let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
	let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
		.expect("Playlists JSON content is not well-formatted !");
	let is_playlist = playlists.iter().position(|playlist| playlist.playlist_name == *playlist_name);
	if is_playlist.is_some() {
		for playlist in playlists {
			if &playlist.playlist_name == playlist_name {
				for song_path in songs_path {
					let song_infos = get_all_metadata(song_path.expect("Songs folder is empty !")
						.path().to_str().expect("Can't transform PathBuf to str !")
						.to_string()
					);
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
