//! Measured wrapping shared by dialogue, narration and menu text.
#[derive(Clone, Debug)]
pub struct Line {
    pub start: usize,
    pub end: usize,
    pub text: String,
}
#[derive(Hash, PartialEq, Eq)]
pub struct Key {
    pub text: String,
    pub size: u16,
    pub width: u32,
    pub font: String,
}
pub fn wrap(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<Line> {
    let chars: Vec<char> = text.chars().collect();
    let mut lines = vec![];
    let mut start = 0;
    while start < chars.len() {
        let mut end = start;
        let mut space = None;
        while end < chars.len() && chars[end] != '\n' {
            let candidate: String = chars[start..=end].iter().collect();
            if measure(&candidate) > width && end > start {
                break;
            }
            if chars[end].is_whitespace() {
                space = Some(end);
            }
            end += 1;
        }
        let overflow = end < chars.len() && chars[end] != '\n';
        let split = if overflow {
            space.filter(|index| *index > start).unwrap_or(end)
        } else {
            end
        };
        let mut trimmed = split;
        while trimmed > start && chars[trimmed - 1].is_whitespace() {
            trimmed -= 1;
        }
        lines.push(Line {
            start,
            end: trimmed,
            text: chars[start..trimmed].iter().collect(),
        });
        start = split;
        if start < chars.len() && chars[start] == '\n' {
            start += 1;
        } else {
            while start < chars.len() && chars[start].is_whitespace() && chars[start] != '\n' {
                start += 1;
            }
        }
    }
    lines
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wraps_words_and_unbroken_unicode() {
        for text in [
            "one two three four",
            "非常長的沒有空格的對話",
            "supercalifragilistic",
        ] {
            let lines = wrap(text, 5., |s| s.chars().count() as f32);
            assert!(lines.iter().all(|l| l.text.chars().count() <= 5));
            assert!(lines.iter().all(|l| l.end >= l.start));
        }
    }
    #[test]
    fn reveal_ranges_stay_fixed() {
        let lines = wrap("Hello brave world", 10., |s| s.len() as f32);
        assert_eq!(lines[1].text, "brave");
        assert_eq!(lines[1].start, 6);
        assert_eq!(lines[2].start, 12);
    }
}
