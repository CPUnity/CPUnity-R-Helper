//! The Void's script: on the Info tab, while The Void is on, the text is shown as an invented
//! alphabet of stroke-drawn signs. Every letter is always the same sign, so the script can be
//! learned. Hover a word and it decodes into plain letters; leave it and it goes back.
//!
//! This file is the part without drawing (so it is tested): which signs, and how far along a word
//! is. `ui/script_text.rs` paints it.

/// A point in a sign's own box: x and y from 0 to 1, y down.
pub type Pt = (f32, f32);

/// Where a sign's strokes can start and end: the corners, the middles of the sides and the middle.
const ANCHORS: [Pt; 9] = [
    (0.1, 0.0),
    (0.9, 0.0),
    (0.1, 1.0),
    (0.9, 1.0),
    (0.1, 0.5),
    (0.9, 0.5),
    (0.5, 0.5),
    (0.5, 0.0),
    (0.5, 1.0),
];

fn hash(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA6B).rotate_left(11);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Whether `c` is turned into a sign: every letter, digit and punctuation mark. A mark belongs to the
/// word it stands against, so it is hidden with it and decodes with it (a stray plain dot would give
/// a hidden sentence away). Only spaces and the like are left alone.
pub fn is_sign(c: char) -> bool {
    !c.is_whitespace() && !c.is_control()
}

/// The sign for a letter, digit or punctuation mark: a few straight strokes between the anchors,
/// and sometimes a dot. Capitals and small letters share a sign.
pub fn sign(c: char) -> (Vec<(Pt, Pt)>, Option<Pt>) {
    let n = c.to_ascii_lowercase() as u32;
    let count = 2 + (hash(n, 1) * 3.0) as u32;
    let mut strokes = Vec::new();
    for i in 0..count {
        let a = (hash(n, 10 + i * 2) * ANCHORS.len() as f32) as usize % ANCHORS.len();
        let mut b = (hash(n, 11 + i * 2) * ANCHORS.len() as f32) as usize % ANCHORS.len();
        if b == a {
            b = (b + 3) % ANCHORS.len();
        }
        strokes.push((ANCHORS[a], ANCHORS[b]));
    }
    let dot = (hash(n, 40) > 0.5).then(|| (0.1 + 0.8 * hash(n, 41), if hash(n, 42) > 0.5 { 0.25 } else { 0.75 }));
    (strokes, dot)
}

/// Seconds a word takes to decode when the pointer is on it, and to go back once it has left.
pub const DECODE_IN: f32 = 0.32;
pub const ENCODE_OUT: f32 = 0.7;

/// How far along a word is, 0 (all signs) to 1 (all letters), after `dt` seconds more.
pub fn step_amount(amount: f32, hovered: bool, dt: f32) -> f32 {
    let dt = dt.clamp(0.0, 0.1);
    if hovered {
        (amount + dt / DECODE_IN).min(1.0)
    } else {
        (amount - dt / ENCODE_OUT).max(0.0)
    }
}

/// Breaks `text` into lines of at most `cols` letters at the spaces (a word longer than that gets a
/// line to itself). Every letter of the text, in order, is in exactly one line.
pub fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        let n = word.chars().count();
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + n <= cols => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Letter {
    /// Still a sign.
    Sign,
    /// On its way: flickering through other signs.
    Flicker(char),
    /// The plain letter.
    Plain,
}

/// What letter `i` of a word of `n` letters is at `amount`. The letters decode from the left, with
/// two of them flickering at the front.
pub fn letter(i: usize, n: usize, amount: f32, time: f32) -> Letter {
    if amount <= 0.0 {
        return Letter::Sign;
    }
    if amount >= 1.0 {
        return Letter::Plain;
    }
    let front = amount * (n as f32 + 2.0);
    let i = i as f32;
    if i + 1.0 <= front - 1.0 {
        Letter::Plain
    } else if i < front {
        let frame = (time * 18.0) as u32;
        let k = hash(i as u32 + 77, frame);
        Letter::Flicker((b'a' + (k * 26.0) as u8 % 26) as char)
    } else {
        Letter::Sign
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_letter_is_always_the_same_sign_and_capitals_match_small_letters() {
        for c in ('a'..='z').chain('0'..='9') {
            assert_eq!(sign(c), sign(c));
            assert_eq!(sign(c), sign(c.to_ascii_uppercase()));
            let (strokes, dot) = sign(c);
            assert!((2..=4).contains(&strokes.len()));
            for (a, b) in &strokes {
                assert!(a != b, "a stroke has some length");
                for p in [a, b] {
                    assert!((0.0..=1.0).contains(&p.0) && (0.0..=1.0).contains(&p.1));
                }
            }
            if let Some(d) = dot {
                assert!((0.0..=1.0).contains(&d.0) && (0.0..=1.0).contains(&d.1));
            }
        }
    }

    #[test]
    fn nearly_every_letter_has_a_sign_of_its_own_so_the_script_can_be_read() {
        let mut seen = std::collections::HashSet::new();
        for c in 'a'..='z' {
            let (s, d) = sign(c);
            let key = format!("{:?}{:?}", s.iter().map(|(a, b)| ((a.0 * 10.0) as i32, (a.1 * 10.0) as i32, (b.0 * 10.0) as i32, (b.1 * 10.0) as i32)).collect::<Vec<_>>(), d.map(|p| ((p.0 * 10.0) as i32, (p.1 * 10.0) as i32)));
            seen.insert(key);
        }
        assert!(seen.len() >= 25, "{} different signs for 26 letters", seen.len());
    }

    #[test]
    fn letters_digits_and_punctuation_all_become_signs_and_only_spaces_stay() {
        for c in "azAZ09.,:;-()'\"!?/&%+*#@\u{2019}\u{2014}\u{2026}é".chars() {
            assert!(is_sign(c), "{c:?}");
        }
        for c in [' ', '\t', '\n', '\u{a0}'] {
            assert!(!is_sign(c), "{c:?}");
        }
    }

    #[test]
    fn a_punctuation_mark_has_a_sign_like_any_letter_and_none_is_the_same_as_another() {
        let marks: Vec<char> = ".,:;-()'\"!?/&%+*#@\u{2019}\u{2014}\u{2026}".chars().collect();
        let all: Vec<char> = ('a'..='z').chain('0'..='9').chain(marks.iter().copied()).collect();
        for c in &marks {
            let (strokes, dot) = sign(*c);
            assert!((2..=4).contains(&strokes.len()), "{c:?}");
            assert!(strokes.iter().all(|(a, b)| a != b));
            assert!(dot.map_or(true, |d| (0.0..=1.0).contains(&d.0) && (0.0..=1.0).contains(&d.1)));
        }
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(sign(*a), sign(*b), "{a:?} and {b:?} look the same");
            }
        }
        // a stray dot looks like the rest of the text: as many strokes as any letter
        assert!(sign('.').0.len() >= 2);
    }

    #[test]
    fn a_word_decodes_while_hovered_and_goes_back_slower_when_left() {
        let mut a = 0.0;
        let mut frames_in = 0;
        while a < 1.0 {
            a = step_amount(a, true, 1.0 / 60.0);
            frames_in += 1;
            assert!(frames_in < 600);
        }
        let seconds_in = frames_in as f32 / 60.0;
        assert!((seconds_in - DECODE_IN).abs() < 0.05, "{seconds_in}");
        let mut frames_out = 0;
        while a > 0.0 {
            a = step_amount(a, false, 1.0 / 60.0);
            frames_out += 1;
            assert!(frames_out < 600);
        }
        assert!(frames_out as f32 / 60.0 > seconds_in, "it lingers a little before it hides again");
        // a long frame does not jump it
        assert!(step_amount(0.0, true, 5.0) <= 0.1 / DECODE_IN + 1e-6);
        assert_eq!(step_amount(0.5, true, 0.0), 0.5);
    }

    #[test]
    fn letters_decode_from_the_left_with_a_flicker_at_the_front() {
        let n = 8;
        assert!((0..n).all(|i| letter(i, n, 0.0, 1.0) == Letter::Sign));
        assert!((0..n).all(|i| letter(i, n, 1.0, 1.0) == Letter::Plain));
        for step in 1..20 {
            let amount = step as f32 / 20.0;
            let states: Vec<Letter> = (0..n).map(|i| letter(i, n, amount, 3.0)).collect();
            // plain letters first, then flickers, then signs: never mixed up
            let rank = |l: &Letter| match l {
                Letter::Plain => 0,
                Letter::Flicker(_) => 1,
                Letter::Sign => 2,
            };
            assert!(states.windows(2).all(|w| rank(&w[0]) <= rank(&w[1])), "{states:?}");
            assert!(states.iter().filter(|l| matches!(l, Letter::Flicker(_))).count() <= 2);
            for l in &states {
                if let Letter::Flicker(c) = l {
                    assert!(c.is_ascii_lowercase());
                }
            }
        }
        // the front flickers: different signs a moment later
        let at = |t: f32| letter(0, 1, 0.3, t);
        assert!((0..40).map(|k| at(k as f32 * 0.07)).collect::<std::collections::HashSet<_>>().len() > 3);
        // one-letter words work
        assert_eq!(letter(0, 1, 1.0, 0.0), Letter::Plain);
    }

    #[test]
    fn text_wraps_at_spaces_and_keeps_every_letter_in_order() {
        let text = "So you ended Heaven and started The Void? At least I quit smoking because of you.";
        for cols in [10, 20, 26, 40, 200] {
            let lines = wrap(text, cols);
            assert_eq!(lines.join(" "), text);
            assert!(lines.iter().all(|l| l.chars().count() <= cols.max(10)), "{cols}: {lines:?}");
        }
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        // a word that is too long stands alone
        assert_eq!(wrap("a extraordinarily b", 5), vec!["a", "extraordinarily", "b"]);
        assert!(wrap("", 10).is_empty());
        assert!(wrap("   ", 10).is_empty());
    }
}
