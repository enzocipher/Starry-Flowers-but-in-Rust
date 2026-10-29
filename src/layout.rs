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
    let mut prefix = String::new();
    while start < chars.len() {
        let mut end = start;
        let mut space = None;
        prefix.clear();
        while end < chars.len() && chars[end] != '\n' {
            prefix.push(chars[end]);
            if measure(&prefix) > width && end > start {
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
/// Linear wrapping for renderers whose glyph advances are additive (no kerning).
#[allow(dead_code)] // Kept for render backends that can supply additive advances.
pub fn wrap_advances(text: &str, width: f32, advance: impl Fn(char) -> f32) -> Vec<Line> {
    let chars: Vec<char> = text.chars().collect();
    let mut lines = vec![];
    let mut start = 0;
    while start < chars.len() {
        let mut end = start;
        let mut space = None;
        let mut measured = 0.;
        while end < chars.len() && chars[end] != '\n' {
            measured += advance(chars[end]);
            if measured > width && end > start {
                break;
            }
            if chars[end].is_whitespace() {
                space = Some(end);
            }
            end += 1;
        }
        let overflow = end < chars.len() && chars[end] != '\n';
        let split = if overflow {
            space.filter(|i| *i > start).unwrap_or(end)
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
    fn additive_wrap_preserves_breaks_and_reveal_ranges() {
        for text in [
            "",
            "Hello brave world",
            "\n\n one  two\nthree",
            "a\nb\n",
            "非常長的沒有空格的對話",
            "Español ❤\u{fe0f} 💙",
            "longunbrokenword followed by words",
        ] {
            for width in [0., 1., 5., 10., 30.] {
                let advance = |c: char| {
                    if c == '\u{fe0f}' {
                        0.
                    } else {
                        (c as u32 % 3 + 1) as f32
                    }
                };
                let old = wrap(text, width, |s| s.chars().map(advance).sum());
                let new = wrap_advances(text, width, advance);
                assert_eq!(
                    old.iter()
                        .map(|l| (&l.text, l.start, l.end))
                        .collect::<Vec<_>>(),
                    new.iter()
                        .map(|l| (&l.text, l.start, l.end))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
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
    fn wrap_measures_each_prefix_once() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let text = "long unbroken paragraph";
        wrap(text, 6., |s| {
            calls.set(calls.get() + 1);
            s.chars().count() as f32
        });
        assert!(calls.get() <= text.chars().count() * 2);
    }
    #[test]
    fn reveal_ranges_stay_fixed() {
        let lines = wrap("Hello brave world", 10., |s| s.len() as f32);
        assert_eq!(lines[1].text, "brave");
        assert_eq!(lines[1].start, 6);
        assert_eq!(lines[2].start, 12);
    }
}
