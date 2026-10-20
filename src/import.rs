//! Adding a song: read the mp3, listen to it (chartgen.rs) and keep a copy with its charts (songs.rs).
//! It takes a few seconds for a long song, so main.rs runs it on a thread of its own.
use crate::{audio, chartgen, songs};
use std::path::Path;

/// Adds the mp3 at `path` to the songs in `dir`. The error is something to tell the player.
pub fn add(dir: &Path, path: &str) -> Result<songs::Item, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Could not read the file: {e}"))?;
    if bytes.len() > songs::MAX_BYTES {
        return Err("That file is too big (the limit is 60 MB).".to_string());
    }
    let (samples, rate) = audio::decode_file(bytes.clone()).ok_or("Could not read that as an mp3 (or it is longer than 15 minutes).")?;
    let song = chartgen::make_song(&songs::title_of(path), "your song", &samples, rate)
        .ok_or("That is too short (under 20 seconds), too long, or silent.")?;
    songs::save(dir, &bytes, &song).map_err(|e| format!("Could not keep the song: {e}"))
}
