use std::fs::{self, File, read_to_string};
use std::io::{BufWriter, Write};

use crate::api::{Playlist};

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
			let _ = serde_json::to_writer_pretty(&mut settings_writer, &settings);
			let _ = settings_writer.flush();
			break;
		}
	}
}

// Verify that playlists.json exists or create it
pub fn verify_files() {
	if !fs::exists("playlists.json").expect("Non authorized folder check !") {
		let mut new: Vec<Playlist> = Vec::new();
		new.push(Playlist { playlist_name: "All songs".to_string(), songs_list: Vec::new() });
		new.push(Playlist { playlist_name: "Favorites".to_string(), songs_list: Vec::new() });

		let playlists_file = File::create("playlists.json").expect("Failed to create/open playlists.json");
		let mut playlists_writer = BufWriter::new(playlists_file);
		let _ = serde_json::to_writer_pretty(&mut playlists_writer, &new);
		let _ = playlists_writer.flush();
	}
}

// Convert seconds to minutes/seconds
pub fn seconds_to_minsec(seconds: f64) -> (u32, u32) {
	let min = (seconds / 60.0).floor() as u32;
	let sec = (seconds % 60.0).round() as u32;

	(min, sec)
}
