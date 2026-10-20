//! The songs Game 3 can play besides the story's two: the player's own mp3 files. Adding one makes
//! its charts (see chartgen.rs) and keeps both in the program's folder (`game3-songs` next to the
//! settings): `<id>.mp3` is a copy of the file and `<id>.chart` its notes, in the same text format as
//! the story's (assets/game3-charts.txt). The id is made from the file's contents, so adding the same
//! song twice is the same song. Nothing here makes sound or draws anything.

use crate::game3::{self, parse_charts, Song};
use std::path::{Path, PathBuf};

/// The biggest mp3 that is taken (bytes).
pub const MAX_BYTES: usize = 60 * 1024 * 1024;

/// Where the songs are kept.
pub fn dir() -> PathBuf {
    crate::config::config_dir().join("game3-songs")
}

/// Where a song's sound comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// One of the story's two songs, which are in the program.
    Story(usize),
    /// A file in the songs folder.
    File(PathBuf),
}

/// A song that can be played: its id (the playlist keeps these), its notes and where its sound is.
#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub song: Song,
    pub source: Source,
}

impl Item {
    /// Whether the player added it (and so can take it away again).
    pub fn imported(&self) -> bool {
        matches!(self.source, Source::File(_))
    }
}

/// An id for these bytes: 16 hex digits of their FNV-1a hash.
pub fn id_of(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The story's two songs, which are always there once the Underworld is open.
pub fn story_items() -> Vec<Item> {
    game3::songs().iter().enumerate().map(|(i, s)| Item { id: format!("story-{}", i + 1), song: s.clone(), source: Source::Story(i) }).collect()
}

/// The songs in `dir` that have a chart and a sound, by title.
pub fn load(dir: &Path) -> Vec<Item> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut items: Vec<Item> = read
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("chart") {
                return None;
            }
            let id = path.file_stem()?.to_str()?.to_string();
            if !crate::games::valid_id(&id) {
                return None;
            }
            let mp3 = dir.join(format!("{id}.mp3"));
            if !mp3.is_file() {
                return None;
            }
            let song = parse_charts(&std::fs::read_to_string(&path).ok()?).into_iter().next()?;
            if song.len <= 0.0 || crate::games::Difficulty::ALL.iter().all(|d| song.notes(*d).is_empty()) {
                return None;
            }
            Some(Item { id, song, source: Source::File(mp3) })
        })
        .collect();
    items.sort_by(|a, b| a.song.title.to_lowercase().cmp(&b.song.title.to_lowercase()).then(a.id.cmp(&b.id)));
    items
}

/// Everything that can be played: the story's songs, then the player's.
pub fn library(dir: &Path) -> Vec<Item> {
    let mut all = story_items();
    all.extend(load(dir));
    all
}

/// Keeps a song: a copy of the mp3 and its chart. Returns it as it is in the library now.
pub fn save(dir: &Path, mp3: &[u8], song: &Song) -> std::io::Result<Item> {
    std::fs::create_dir_all(dir)?;
    let id = id_of(mp3);
    let file = dir.join(format!("{id}.mp3"));
    std::fs::write(&file, mp3)?;
    std::fs::write(dir.join(format!("{id}.chart")), song.to_text())?;
    Ok(Item { id, song: song.clone(), source: Source::File(file) })
}

/// Takes a song away (both files). Story songs and bad ids are left alone.
pub fn remove(dir: &Path, id: &str) {
    if id.starts_with("story-") || !crate::games::valid_id(id) {
        return;
    }
    let _ = std::fs::remove_file(dir.join(format!("{id}.mp3")));
    let _ = std::fs::remove_file(dir.join(format!("{id}.chart")));
}

/// The title a file name gives a song: the name without its folder and its extension.
pub fn title_of(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let stem = match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    };
    let t = stem.trim();
    if t.is_empty() {
        "Untitled".to_string()
    } else {
        t.chars().take(80).collect()
    }
}

/// `secs` as `m:ss`.
pub fn clock(secs: f32) -> String {
    let s = secs.max(0.0).round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game3::Note;

    fn song(title: &str) -> Song {
        let n = |t, lane| Note { t, lane };
        Song::from_parts(title, "your song", 120.0, 0.5, 90.0, [vec![n(1.0, 0)], vec![n(1.0, 1), n(2.0, 5)], vec![n(1.0, 7)]])
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cpunity-songs-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn the_id_comes_from_the_contents() {
        assert_eq!(id_of(b"abc"), id_of(b"abc"));
        assert_ne!(id_of(b"abc"), id_of(b"abd"));
        assert_eq!(id_of(b"").len(), 16);
        assert!(crate::games::valid_id(&id_of(b"anything")));
    }

    #[test]
    fn the_story_songs_are_in_the_library_and_cannot_be_removed() {
        let items = story_items();
        assert_eq!(items.len(), game3::SONGS);
        assert_eq!(items[1].id, "story-2");
        assert!(items.iter().all(|i| !i.imported()));
        remove(&temp("none"), "story-1");
    }

    #[test]
    fn a_saved_song_is_listed_with_its_notes_and_can_be_removed() {
        let dir = temp("save");
        assert!(load(&dir).is_empty(), "no folder yet");
        let a = save(&dir, b"first mp3", &song("Zebra")).unwrap();
        let b = save(&dir, b"second mp3", &song("apple")).unwrap();
        assert_ne!(a.id, b.id);
        assert!(a.imported());
        let again = save(&dir, b"first mp3", &song("Zebra")).unwrap();
        assert_eq!(again.id, a.id, "the same file is the same song");
        let items = load(&dir);
        assert_eq!(items.iter().map(|i| i.song.title.as_str()).collect::<Vec<_>>(), vec!["apple", "Zebra"], "by title, ignoring case");
        let z = items.iter().find(|i| i.id == a.id).unwrap();
        assert_eq!(z.song.notes(crate::games::Difficulty::Normal).len(), 2);
        assert_eq!(std::fs::read(match &z.source {
            Source::File(p) => p.clone(),
            _ => panic!(),
        })
        .unwrap(), b"first mp3");
        let all = library(&dir);
        assert_eq!(all.len(), game3::SONGS + 2);
        assert!(all[0].id.starts_with("story-"));
        remove(&dir, &a.id);
        assert_eq!(load(&dir).len(), 1);
        remove(&dir, "../../etc");
        assert_eq!(load(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_song_missing_its_sound_or_its_notes_is_not_listed() {
        let dir = temp("broken");
        let a = save(&dir, b"x", &song("Has both")).unwrap();
        std::fs::write(dir.join("deadbeef.chart"), song("No mp3").to_text()).unwrap();
        std::fs::write(dir.join("cafe.mp3"), b"y").unwrap();
        std::fs::write(dir.join("cafe.chart"), "garbage").unwrap();
        std::fs::write(dir.join("not valid.chart"), song("Bad id").to_text()).unwrap();
        let items = load(&dir);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, a.id);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn titles_come_from_file_names() {
        assert_eq!(title_of(r"C:\Music\My Song.mp3"), "My Song");
        assert_eq!(title_of("/home/me/a.b.c.mp3"), "a.b.c");
        assert_eq!(title_of("noext"), "noext");
        assert_eq!(title_of(r"C:\.mp3"), ".mp3");
        assert_eq!(title_of(""), "Untitled");
        assert_eq!(clock(125.4), "2:05");
        assert_eq!(clock(-3.0), "0:00");
    }
}
