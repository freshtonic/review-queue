//! The content and layout of the detail panel.

/// Marks where markup was removed, so that a line with only markup can be dropped.
const REMOVED: char = '\u{0}';

/// The description as plain text lines of at most `width` characters, at most `max_lines`
/// of them. HTML comments, images, and runs of blank lines are removed. If the text is
/// cut, the last line ends with `…`.
pub fn description_lines(body: &str, width: usize, max_lines: usize) -> Vec<String> {
    let width = width.max(1);
    let text = remove_between(body, "![", ")");
    let text = remove_between(&text, "<!--", "-->");
    let text = remove_between(&text, "<img", ">");

    // A line that had only removed markup goes away. Then runs of blank lines become one.
    let mut paragraphs: Vec<&str> = Vec::new();
    for line in text.lines() {
        let kept = line.replace(REMOVED, "");
        let kept = kept.trim();
        if kept.is_empty() && line.contains(REMOVED) {
            continue;
        }
        let blank = kept.is_empty();
        if blank && paragraphs.last().is_none_or(|p| p.is_empty()) {
            continue;
        }
        paragraphs.push(if blank { "" } else { line });
    }
    while paragraphs.last().is_some_and(|p| p.is_empty()) {
        paragraphs.pop();
    }

    let mut lines: Vec<String> = Vec::new();
    for paragraph in paragraphs {
        let paragraph = paragraph.replace(REMOVED, " ");
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        wrap(&paragraph, width, &mut lines);
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            let keep = last.chars().count().min(width - 1);
            *last = last.chars().take(keep).collect::<String>() + "…";
        }
    }
    lines
}

/// Replaces each `start…end` in `text` with one `REMOVED` mark. Without an end, the rest
/// of the text goes.
fn remove_between(text: &str, start: &str, end: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(start) {
        out.push_str(&rest[..i]);
        out.push(REMOVED);
        rest = match rest[i + start.len()..].find(end) {
            Some(j) => &rest[i + start.len() + j + end.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// Wraps the words of `text` into lines of at most `width` characters. A word that is
/// longer than `width` is split.
fn wrap(text: &str, width: usize, lines: &mut Vec<String>) {
    let mut line = String::new();
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        let used = line.chars().count();
        if !line.is_empty() && used + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        while word.len() > width - line.chars().count() {
            let room = width - line.chars().count();
            line.extend(word.drain(..room));
            lines.push(std::mem::take(&mut line));
        }
        line.extend(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
}
