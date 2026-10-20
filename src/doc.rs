//! The tiny text format behind the Changelog and Info tabs: `#` / `##` / `###` headings, `- `
//! bullets (indent two spaces per level), plain paragraphs, and `**bold**`. Pure text handling,
//! no drawing.

/// The change log that is built into the program (shown on the Changelog tab).
pub const CHANGELOG: &str = include_str!("../CHANGELOG.md");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// `# text`
    Title(String),
    /// `## text` (in the changelog: one per version)
    Heading(String),
    /// `### text`
    Subheading(String),
    /// `- text`, nested by indentation
    Item { level: u8, text: String },
    /// Plain text; consecutive lines are one paragraph.
    Paragraph(String),
    /// `@copy Label | text`: text that is copied to the clipboard when clicked.
    Copy { label: String, text: String },
    /// `@link Label | https://...`: a world button that opens the page in the browser.
    Link { label: String, url: String },
}

/// `Label | value` of an `@copy` / `@link` line; both parts must be there.
fn label_and_value(rest: &str) -> Option<(String, String)> {
    let (label, value) = rest.split_once('|')?;
    let (label, value) = (label.trim(), value.trim());
    (!label.is_empty() && !value.is_empty()).then(|| (label.to_string(), value.to_string()))
}

pub fn parse(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    // the last block can take more lines (a wrapped bullet or paragraph)
    let mut open = false;
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            open = false;
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent == 0 {
            let heading = if let Some(t) = trimmed.strip_prefix("### ") {
                Some(Block::Subheading(t.trim().to_string()))
            } else if let Some(t) = trimmed.strip_prefix("## ") {
                Some(Block::Heading(t.trim().to_string()))
            } else {
                trimmed.strip_prefix("# ").map(|t| Block::Title(t.trim().to_string()))
            };
            if let Some(block) = heading {
                blocks.push(block);
                open = false;
                continue;
            }
            let action = if let Some(rest) = trimmed.strip_prefix("@copy ") {
                label_and_value(rest).map(|(label, text)| Block::Copy { label, text })
            } else if let Some(rest) = trimmed.strip_prefix("@link ") {
                label_and_value(rest).map(|(label, url)| Block::Link { label, url })
            } else {
                None
            };
            if let Some(block) = action {
                blocks.push(block);
                open = false;
                continue;
            }
        }
        if let Some(t) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            blocks
                .push(Block::Item { level: (indent / 2).min(3) as u8, text: t.trim().to_string() });
            open = true;
            continue;
        }
        if open {
            if let Some(Block::Item { text, .. } | Block::Paragraph(text)) = blocks.last_mut() {
                text.push(' ');
                text.push_str(trimmed);
                continue;
            }
        }
        blocks.push(Block::Paragraph(trimmed.to_string()));
        open = true;
    }
    blocks
}

/// `text` cut into (piece, bold) parts at the `**` marks; backticks are dropped.
pub fn spans(text: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut bold = false;
    for (i, part) in text.split("**").enumerate() {
        if i > 0 {
            bold = !bold;
        }
        let clean = part.replace('`', "");
        if !clean.is_empty() {
            out.push((clean, bold));
        }
    }
    out
}

/// The blocks that belong to each `##` heading (anything before the first one is dropped, which
/// is the `# Title`).
pub fn sections(blocks: &[Block]) -> Vec<(&str, &[Block])> {
    let starts: Vec<usize> = blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| matches!(b, Block::Heading(_)))
        .map(|(i, _)| i)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(n, &start)| {
            let end = starts.get(n + 1).copied().unwrap_or(blocks.len());
            let Block::Heading(title) = &blocks[start] else { unreachable!() };
            (title.as_str(), &blocks[start + 1..end])
        })
        .collect()
}

/// A `##` heading that starts with this marker is shown as a closed drop-down (Info tab).
pub const DROPDOWN_MARK: &str = "[+]";

/// What to draw: a block as it is, or a drop-down with its body.
#[derive(Debug, PartialEq)]
pub enum Part<'a> {
    Block(&'a Block),
    Dropdown { title: &'a str, body: &'a [Block] },
}

/// The blocks in order, with every `## [+] Title` heading and the blocks under it (up to the next
/// title or heading) turned into a drop-down.
pub fn parts(blocks: &[Block]) -> Vec<Part<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        if let Block::Heading(text) = &blocks[i] {
            if let Some(title) = text.strip_prefix(DROPDOWN_MARK) {
                let end = blocks[i + 1..]
                    .iter()
                    .position(|b| matches!(b, Block::Title(_) | Block::Heading(_)))
                    .map_or(blocks.len(), |n| i + 1 + n);
                out.push(Part::Dropdown { title: title.trim(), body: &blocks[i + 1..end] });
                i = end;
                continue;
            }
        }
        out.push(Part::Block(&blocks[i]));
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\n## 0.6.0 - 2026-10-17\n\n### Added\n- **Files tab**: tiles\n  that wrap onto a second line.\n  - nested point\n- second\n\nA paragraph\nover two lines.\n\n## 0.5.1\n- old\n";

    #[test]
    fn copy_and_link_lines_become_their_own_blocks() {
        let blocks = parse(
            "Reach me:\n@copy Email | a@b.example\n@link GitHub page | https://github.com/x/y\n@copy broken\n",
        );
        assert_eq!(blocks[0], Block::Paragraph("Reach me:".into()));
        assert_eq!(blocks[1], Block::Copy { label: "Email".into(), text: "a@b.example".into() });
        assert_eq!(
            blocks[2],
            Block::Link { label: "GitHub page".into(), url: "https://github.com/x/y".into() }
        );
        // a line without "label | value" stays plain text
        assert_eq!(blocks[3], Block::Paragraph("@copy broken".into()));
        assert_eq!(blocks.len(), 4);
    }

    #[test]
    fn marked_headings_become_drop_downs() {
        let blocks = parse("# T\nHello\n## Plain\n- a\n## [+] P.S\nShh.\nMore.\n- b\n# Next\nEnd");
        let parts = parts(&blocks);
        // Title, Hello, Plain heading, bullet, drop-down, Next title, End
        assert_eq!(parts.len(), 7);
        assert!(matches!(parts[2], Part::Block(Block::Heading(_))));
        match &parts[4] {
            Part::Dropdown { title, body } => {
                assert_eq!(*title, "P.S");
                assert_eq!(body.len(), 2, "paragraph and bullet");
            }
            other => panic!("expected a drop-down, got {other:?}"),
        }
        assert!(matches!(parts[5], Part::Block(Block::Title(_))));
        // a marked heading at the very end takes the rest of the text
        let tail = parse("## [+] Last\nOnly this.");
        assert!(matches!(&parts_of(&tail)[..], [Part::Dropdown { title: "Last", .. }]));
    }

    fn parts_of(blocks: &[Block]) -> Vec<Part<'_>> {
        super::parts(blocks)
    }

    #[test]
    fn text_is_cut_into_blocks() {
        let blocks = parse(SAMPLE);
        assert_eq!(blocks[0], Block::Title("Changelog".into()));
        assert_eq!(blocks[1], Block::Heading("0.6.0 - 2026-10-17".into()));
        assert_eq!(blocks[2], Block::Subheading("Added".into()));
        assert_eq!(
            blocks[3],
            Block::Item {
                level: 0,
                text: "**Files tab**: tiles that wrap onto a second line.".into()
            }
        );
        assert_eq!(blocks[4], Block::Item { level: 1, text: "nested point".into() });
        assert_eq!(blocks[5], Block::Item { level: 0, text: "second".into() });
        assert_eq!(blocks[6], Block::Paragraph("A paragraph over two lines.".into()));
        assert_eq!(blocks[7], Block::Heading("0.5.1".into()));
        assert_eq!(blocks.len(), 9);
        // a leading byte order mark (Notepad) is ignored
        assert_eq!(parse("\u{feff}# Hi")[0], Block::Title("Hi".into()));
    }

    #[test]
    fn bold_marks_and_backticks() {
        assert_eq!(
            spans("a **b** `c`"),
            vec![("a ".to_string(), false), ("b".to_string(), true), (" c".to_string(), false)]
        );
        assert!(spans("").is_empty());
    }

    #[test]
    fn headings_group_their_blocks() {
        let blocks = parse(SAMPLE);
        let sections = sections(&blocks);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "0.6.0 - 2026-10-17");
        assert_eq!(sections[0].1.len(), 5);
        assert_eq!(sections[1].0, "0.5.1");
        assert_eq!(sections[1].1.len(), 1);
    }

    #[test]
    fn the_real_changelog_starts_with_the_new_version() {
        let blocks = parse(CHANGELOG);
        let sections = sections(&blocks);
        // a released version with its date, not "Unreleased"
        let first = sections[0].0;
        assert!(
            first.starts_with(|c: char| c.is_ascii_digit()) && first.contains(" - 20"),
            "{}",
            first
        );
        assert!(sections.iter().any(|(title, _)| title.starts_with("0.5.1")));
        assert!(sections[0].1.len() > 10);
    }
}
