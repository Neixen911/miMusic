use serde::{Deserialize, Serialize};
use std::fs::{File, read_to_string};
use std::io::{BufWriter, Write};

use crate::settings;

#[derive(Clone, Deserialize, Serialize)]
pub struct Playlist {
	pub playlist_name: String,
    pub songs_list: Vec<String>,
}

// Add OR Remove song from a playlist
pub fn toggle_playlists(path: String, selected_playlist: &String) {
	let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
	let mut playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
		.expect("Playlists JSON content is not well-formatted !");
	for playlist in &mut playlists {
		if &playlist.playlist_name == selected_playlist {
			if playlist.songs_list.contains(&path) {
				// Delete song from the playlist
				let position = playlist.songs_list.iter().position(|n| *n == path).expect("Can't get position of path into JSON file !");
				playlist.songs_list.swap_remove(position);
			} else {
				// Add song to the playlist
				playlist.songs_list.push(path);
			}

			let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
			let mut playlists_writer = BufWriter::new(playlists_file);
			let _ = serde_json::to_writer_pretty(&mut playlists_writer, &playlists);
			let _ = playlists_writer.flush();
			break;
		}
	}
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
	let _ = serde_json::to_writer_pretty(&mut playlists_writer, &playlists);
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
	let _ = serde_json::to_writer_pretty(&mut playlists_writer, &playlists);
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
	let _ = serde_json::to_writer_pretty(&mut playlists_writer, &playlists);
	let _ = playlists_writer.flush();
}

// Return all playlists datas
pub fn get_all_playlists() -> Vec<Playlist> {
	settings::verify_files();
	let playlists_content = read_to_string("playlists.json").expect("Can't read content of playlists.json file !");
	let playlists: Vec<Playlist> = serde_json::from_str(&playlists_content)
		.expect("Playlists JSON content is not well-formatted !");

	playlists
}