use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, read_to_string, remove_file};
use std::io::{BufWriter, Write};

use crate::api;

#[derive(Deserialize, Serialize)]
pub struct Playlist {
	pub playlist_name: String,
    pub songs_list: Vec<String>,
}

// Update a setting value in settings file
pub fn update_settings(key: String, new_value: f32) {
	let settings_content = read_to_string("settings.json").expect("Can't read content of settings.json file !");
	let mut settings: Vec<(String, f32)> = serde_json::from_str(&settings_content)
		.expect("Settings JSON content is not well-formatted !");
	for (setting_key, setting_value) in &mut settings {
		if key == *setting_key {
			*setting_value = new_value;

			let settings_file = File::create("settings.json").expect("Failed to create/open settings.json");
			let mut settings_writer = BufWriter::new(settings_file);
			let _ = serde_json::to_writer(&mut settings_writer, &settings);
			let _ = settings_writer.flush();
			break;
		}
	}
}

pub fn set_favorites(path: &str) {
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
pub fn get_all_songs_from_active_playlist(playlist_name: &String) -> Vec<HashMap<String, String>> {
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
					let song_infos = api::get_song_infos_from_file(song_path.expect("Songs folder is empty !").path().to_str().expect("Unable to convert to str"));
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
pub fn get_all_playlists() -> Vec<Playlist> {
	let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
	let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
		.expect("Playlists JSON content is not well-formatted !");

	playlists
}

// Add a new playlist
pub fn add_playlist() {
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
pub fn modify_playlist(actual_playlist_position: usize, new_playlist_name: &String) {
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
pub fn remove_playlist(playlist_position_to_remove: usize) {
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
pub fn add_or_remove_song_to_playlist(song_to_add: String, selected_playlist: &String) {
	let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
	let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
		.expect("Playlists JSON content is not well-formatted !");
	for playlist in &mut playlists {
		if &playlist.playlist_name == selected_playlist {
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
	let _ = serde_json::to_writer(&mut playlists_writer, &playlists);
	let _ = playlists_writer.flush();

	// Remove song itself
	let _ = remove_file(&song_to_remove);
}
