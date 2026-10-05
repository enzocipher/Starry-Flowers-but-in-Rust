#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod engine;
mod layout;
mod preferences;
use engine::*;
use macroquad::{audio::*, prelude::*};
use preferences::{punch_offset, visible_text, Preferences};
use serde::Serialize;
use std::{collections::HashMap, path::PathBuf};
const INK: Color = Color::new(0.22, 0.21, 0.36, 1.0);
const BLUE: Color = Color::new(0.37, 0.55, 0.85, 1.0);
#[cfg(test)]
mod render_tests {
    use super::*;
    #[test]
    fn narrative_visuals_resolve_to_original_assets() {
        let story = Story::load();
        for label in [
            "start", "ch0", "ch4b", "ch7b", "ex1", "ex2", "ex3", "ex4", "ex5",
        ] {
            let mut state = State::default();
            state.start(&story, label);
            for _ in 0..10000 {
                let stop = state.run(&story);
                for sprite in &state.sprites {
                    if [
                        "black",
                        "white",
                        "title",
                        "shootingstars",
                        "firststar",
                        "petals",
                        "cinebars",
                        "lyrics",
                        "tld",
                        "cred1",
                        "cred2",
                        "cred6",
                        "cred7",
                        "credscroll",
                        "credflowers",
                    ]
                    .contains(&sprite.tag.as_str())
                    {
                        continue;
                    }
                    let names = layers(sprite, &state, &story);
                    assert!(!names.is_empty(), "Unresolved: {sprite:?}");
                    for name in names {
                        assert!(
                            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("assets")
                                .join(&story.images[&name])
                                .is_file(),
                            "Missing {name}"
                        );
                    }
                }
                match stop {
                    Stop::Menu => state.pc = story.ops[state.pc - 1].choices[1].target,
                    Stop::End => break,
                    _ => {}
                }
            }
        }
        for group in &story.accessories {
            for name in group {
                assert!(story.images.contains_key(name), "Missing accessory {name}");
            }
        }
    }
}
fn conf() -> Conf {
    let icon = Image::from_file_with_format(include_bytes!("../assets/gui/window_icon.png"), None)
        .ok()
        .map(|image| {
            let mut icon = macroquad::miniquad::conf::Icon {
                small: [0; 16 * 16 * 4],
                medium: [0; 32 * 32 * 4],
                big: [0; 64 * 64 * 4],
            };
            for (pixels, side) in [
                (&mut icon.small[..], 16usize),
                (&mut icon.medium[..], 32),
                (&mut icon.big[..], 64),
            ] {
                for y in 0..side {
                    for x in 0..side {
                        let source = ((y * image.height as usize / side) * image.width as usize
                            + x * image.width as usize / side)
                            * 4;
                        pixels[(y * side + x) * 4..(y * side + x + 1) * 4]
                            .copy_from_slice(&image.bytes[source..source + 4]);
                    }
                }
            }
            icon
        });
    Conf {
        window_title: "Starry Flowers".into(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        icon,
        ..Default::default()
    }
}
fn save_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("StarryFlowersRust")
}
fn write_json<T: Serialize>(name: &str, value: &T) -> Result<(), String> {
    let dir = save_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{name}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join(name)).map_err(|e| e.to_string())
}
fn read_json<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
    serde_json::from_slice(&std::fs::read(save_dir().join(name)).ok()?).ok()
}
struct Art {
    layouts: std::cell::RefCell<HashMap<layout::Key, Vec<layout::Line>>>,
    textures: HashMap<String, Texture2D>,
    font: Font,
    inline_symbols: HashMap<char, InlineSymbol>,
    sounds: HashMap<String, Sound>,
    music: String,
    stamps: HashMap<String, (String, f64)>,
    animate: bool,
    font_lang: String,
    face_history: HashMap<String, (String, String, f64)>,
    ui_lang: String,
    ui: HashMap<String, String>,
    fonts: HashMap<String, Font>,
}
impl Art {
    async fn font_for(&mut self, lang: &str) {
        if self.font_lang == lang {
            return;
        }
        let file = match lang {
            "ru" | "ukr" => "ru/VDS_New.ttf",
            "zh" => "zh/ResourceHanRoundedCN-Bold.ttf",
            "kor" => "kor/Goyang.ttf",
            "thai" => "thai/BoonJot-Regular.ttf",
            _ => "None/Nunito-Bold.ttf",
        };
        if let Some(font) = self.fonts.get(file) {
            self.font = font.clone();
            self.font_lang = lang.into();
            return;
        }
        if let Ok(font) = load_ttf_font(&format!("assets/tl/{file}")).await {
            self.font = font.clone();
            self.fonts.insert(file.into(), font);
            self.font_lang = lang.into();
        }
    }
    async fn blended_image(&mut self, name: &str, r: Rect, alpha: f32) {
        let mut blend = 1.;
        let mut old = None;
        if let Some((prefix, _)) = name.split_once("_face_") {
            let entry = self.face_history.entry(prefix.into()).or_insert((
                name.into(),
                String::new(),
                get_time(),
            ));
            if entry.0 != name {
                entry.1 = std::mem::replace(&mut entry.0, name.into());
                entry.2 = get_time();
            }
            if self.animate {
                blend = ((get_time() - entry.2) / 0.18).min(1.) as f32;
                if blend < 1. {
                    old = Some(entry.1.clone());
                }
            }
        }
        if let Some(old) = old {
            if let Some(t) = self.textures.get(&old) {
                draw_texture_ex(
                    t,
                    r.x,
                    r.y,
                    Color::new(1., 1., 1., alpha),
                    DrawTextureParams {
                        dest_size: Some(vec2(r.w, r.h)),
                        ..Default::default()
                    },
                );
            }
        }
        if let Some(t) = self.textures.get(name) {
            draw_texture_ex(
                t,
                r.x,
                r.y,
                Color::new(1., 1., 1., alpha * blend),
                DrawTextureParams {
                    dest_size: Some(vec2(r.w, r.h)),
                    ..Default::default()
                },
            );
        }
    }
    fn age(&mut self, s: &Sprite) -> f32 {
        let attrs: Vec<_> = s
            .attrs
            .iter()
            .filter(|attr| {
                s.tag != "pastille"
                    || ["witch", "pj", "date", "manto", "work", "jacket"].contains(&attr.as_str())
            })
            .collect();
        let identity = format!("{attrs:?}:{}", s.position);
        let entry = self
            .stamps
            .entry(s.tag.clone())
            .or_insert((identity.clone(), get_time()));
        if entry.0 != identity {
            *entry = (identity, get_time());
        }
        (get_time() - entry.1) as f32
    }
    fn centered(&self, text: &str, y: f32, size: u16, color: Color) {
        for (i, line) in clean_text(text).split('\n').enumerate() {
            let width = measure_text(line, Some(&self.font), size, 1.).width;
            self.text(
                line,
                (1280. - width) / 2.,
                y + i as f32 * size as f32 * 1.4,
                size,
                color,
            );
        }
    }
    async fn particles(&mut self, kind: &str, story: &Story) {
        let t = get_time() as f32;
        for i in 0..20 {
            let i = i as f32;
            let (name, x, y, rotation) = match kind {
                "shootingstars" | "firststar" => (
                    "shootingstar2",
                    (i * 133. - t * (80. + i * 3.)).rem_euclid(1450.) - 100.,
                    (i * 83.).rem_euclid(700.),
                    0.,
                ),
                "petals" => (
                    if i as i32 % 2 == 0 { "petal" } else { "petal2" },
                    (i * 157. + (t + i).sin() * 50.).rem_euclid(1300.),
                    (i * 89. + t * (25. + i)).rem_euclid(820.) - 80.,
                    t + i,
                ),
                "credflowers" => (
                    "flower1",
                    (i * 151. + t * 12.).rem_euclid(1400.) - 60.,
                    (i * 97. + t * 40.).rem_euclid(820.) - 80.,
                    t + i,
                ),
                _ => (
                    if i as i32 % 2 == 0 {
                        "titlestar"
                    } else {
                        "titleflower2"
                    },
                    (i * 151. + t * 6.).rem_euclid(1400.) - 60.,
                    (i * 97. + t * 18.).rem_euclid(820.) - 80.,
                    t * 0.5 + i,
                ),
            };
            self.ensure(name, story).await;
            if let Some(texture) = self.textures.get(name) {
                draw_texture_ex(
                    texture,
                    x,
                    y,
                    Color::new(1., 1., 1., 0.5),
                    DrawTextureParams {
                        dest_size: Some(vec2(40., 40.)),
                        rotation,
                        ..Default::default()
                    },
                );
            }
            if kind == "firststar" {
                break;
            }
        }
    }
    async fn ensure(&mut self, name: &str, story: &Story) {
        if self.textures.contains_key(name) {
            return;
        }
        if let Some(path) = story.images.get(name) {
            match load_texture(&format!("assets/{path}")).await {
                Ok(t) => {
                    t.set_filter(FilterMode::Linear);
                    self.textures.insert(name.into(), t);
                }
                Err(e) => eprintln!("{path}: {e}"),
            }
        }
    }
    async fn audio(&mut self, state: &mut State, prefs: &Preferences) {
        let volume = if prefs.mute { 0. } else { prefs.volume };
        let sound_volume = if prefs.mute { 0. } else { prefs.sound_volume };
        if self.music != state.music {
            if let Some(s) = self.sounds.get(&self.music) {
                stop_sound(s)
            }
            self.music = state.music.clone();
            if !self.music.is_empty() {
                let path = format!("assets/audio/{}.ogg", self.music);
                if !self.sounds.contains_key(&self.music) {
                    if let Ok(s) = load_sound(&path).await {
                        self.sounds.insert(self.music.clone(), s);
                    }
                }
                if let Some(s) = self.sounds.get(&self.music) {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: true,
                            volume,
                        },
                    )
                }
            }
        }
        if let Some(s) = self.sounds.get(&self.music) {
            set_sound_volume(s, volume)
        }
        for (name, sound) in &self.sounds {
            if name != &self.music {
                set_sound_volume(sound, sound_volume);
            }
        }
        if !state.sound.is_empty() {
            let name = std::mem::take(&mut state.sound);
            if !self.sounds.contains_key(&name) {
                if let Ok(s) = load_sound(&format!("assets/audio/{name}.wav")).await {
                    self.sounds.insert(name.clone(), s);
                }
            }
            if let Some(s) = self.sounds.get(&name) {
                play_sound(
                    s,
                    PlaySoundParams {
                        looped: false,
                        volume: sound_volume,
                    },
                );
            }
        }
    }
    fn image(&self, name: &str, x: f32, y: f32, w: f32, h: f32) {
        if let Some(t) = self.textures.get(name) {
            draw_texture_ex(
                t,
                x,
                y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(w, h)),
                    ..Default::default()
                },
            );
        }
    }
    fn text_width(&self, text: &str, size: u16) -> f32 {
        let mut width = 0.;
        let mut run = String::new();
        for c in text.chars() {
            if c == '\u{fe0f}' {
                continue;
            }
            if let Some(symbol) = self.inline_symbols.get(&c) {
                width += measure_text(&run, Some(&self.font), size, 1.).width
                    + symbol.advance * size as f32;
                run.clear();
            } else {
                run.push(c);
            }
        }
        width + measure_text(&run, Some(&self.font), size, 1.).width
    }
    fn text(&self, s: &str, x: f32, y: f32, size: u16, color: Color) {
        let mut cursor = x;
        let mut run = String::new();
        let draw_run = |run: &str, cursor: f32| {
            draw_text_ex(
                run,
                cursor,
                y,
                TextParams {
                    font: Some(&self.font),
                    font_size: size,
                    color,
                    ..Default::default()
                },
            );
        };
        for c in s.chars() {
            if c == '\u{fe0f}' {
                continue;
            }
            if let Some(symbol) = self.inline_symbols.get(&c) {
                draw_run(&run, cursor);
                cursor += measure_text(&run, Some(&self.font), size, 1.).width;
                run.clear();
                if let Some(texture) = self.textures.get(&symbol.image) {
                    draw_texture_ex(
                        texture,
                        cursor,
                        y + symbol.baseline_top * size as f32,
                        if symbol.tint {
                            color
                        } else {
                            Color::new(1., 1., 1., color.a)
                        },
                        DrawTextureParams {
                            dest_size: Some(vec2(
                                symbol.width * size as f32,
                                symbol.height * size as f32,
                            )),
                            ..Default::default()
                        },
                    );
                }
                cursor += symbol.advance * size as f32;
            } else {
                run.push(c);
            }
        }
        draw_run(&run, cursor);
    }
    fn image_progress(&self, name: &str, r: Rect, fraction: f32) {
        if let Some(t) = self.textures.get(name) {
            let f = fraction.clamp(0., 1.);
            if f > 0. {
                draw_texture_ex(
                    t,
                    r.x,
                    r.y,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(r.w * f, r.h)),
                        source: Some(Rect::new(0., 0., t.width() * f, t.height())),
                        ..Default::default()
                    },
                );
            }
        }
    }
    fn label(&self, text: &str, x: f32, y: f32, width: f32, size: u16, color: Color) {
        let size = (14..=size)
            .rev()
            .find(|size| self.text_width(text, *size) <= width)
            .unwrap_or(14);
        self.text(text, x, y, size, color);
    }
    fn wrap(&self, s: &str, x: f32, y: f32, width: f32, size: u16) -> f32 {
        let lines = self.lines(s, width, size);
        for (i, line) in lines.iter().enumerate() {
            self.text(&line.text, x, y + i as f32 * size as f32 * 1.35, size, INK);
        }
        y + lines.len() as f32 * size as f32 * 1.35
    }
    fn lines(&self, text: &str, width: f32, size: u16) -> Vec<layout::Line> {
        let key = layout::Key {
            text: clean_text(text),
            size,
            width: width.to_bits(),
            font: self.font_lang.clone(),
        };
        if let Some(lines) = self.layouts.borrow().get(&key) {
            return lines.clone();
        }
        let lines = layout::wrap(&key.text, width, |s| self.text_width(s, size));
        let mut cache = self.layouts.borrow_mut();
        if cache.len() > 300 {
            cache.clear();
        }
        cache.insert(key, lines.clone());
        lines
    }
    fn fitted(&self, text: &str, width: f32, height: f32) -> (u16, Vec<layout::Line>) {
        for size in (12..=32).rev() {
            let lines = self.lines(text, width, size);
            if lines.len() as f32 * size as f32 * 1.35 <= height {
                return (size, lines);
            }
        }
        (12, self.lines(text, width, 12))
    }
    fn flow(
        &self,
        lines: &[layout::Line],
        revealed: usize,
        x: f32,
        top: f32,
        size: u16,
        color: Color,
    ) {
        let ascent = measure_text("Ágj", Some(&self.font), size, 1.).offset_y;
        for (i, line) in lines.iter().enumerate() {
            let count = revealed
                .saturating_sub(line.start)
                .min(line.end - line.start);
            let text: String = line.text.chars().take(count).collect();
            self.text(
                &text,
                x,
                top + ascent + i as f32 * size as f32 * 1.35,
                size,
                color,
            );
        }
    }
    fn speaker(&self, who: &str, name: &str, x: f32, top: f32) -> f32 {
        self.speaker_sized(who, name, x, top, 38)
    }
    fn speaker_sized(&self, who: &str, name: &str, x: f32, top: f32, size: u16) -> f32 {
        let (outer, inner) = match who {
            "p" => (0xffdaed, 0xb84d75),
            "w" => (0xbedbff, 0x5083c1),
            "a" => (0xf3daff, 0xa25898),
            "c" => (0xffe3da, 0xa26658),
            "j" => (0xe6daff, 0x6e58a2),
            "k" => (0xfcdaff, 0xa2589d),
            "r" => (0xe0b0d7, 0xa74a7c),
            "u" => (0xeda97e, 0xa76e4a),
            "h" => (0xed7e8b, 0xa43956),
            "g" => (0xffa3d2, 0xad1b52),
            _ => (0xbedbff, 0x5083c1),
        };
        let rgb = |n: u32| Color::from_rgba((n >> 16) as u8, (n >> 8) as u8, n as u8, 255);
        let metrics = measure_text(name, Some(&self.font), size, 1.);
        let baseline = top + metrics.offset_y;
        for (radius, color) in [(8., rgb(outer)), (4., rgb(inner))] {
            for step in 0..32 {
                let radius = radius * size as f32 / 38.;
                let angle = step as f32 * std::f32::consts::TAU / 32.;
                self.text(
                    name,
                    x + angle.cos() * radius,
                    baseline + angle.sin() * radius,
                    size,
                    color,
                );
            }
        }
        self.text(name, x, baseline, size, WHITE);
        metrics.width
    }
}
fn layers(sprite: &Sprite, state: &State, story: &Story) -> Vec<String> {
    if sprite.tag == "pastille" || sprite.tag == "side_peri" {
        let prefix = &sprite.tag;
        let mut names = vec![format!("{prefix}_base")];
        if prefix == "side_peri" {
            names.push(story.accessories[1][state.acc[1]].clone());
        }
        for (group, default) in [("outfit", "witch"), ("face", "smile")] {
            let attr = sprite
                .attrs
                .iter()
                .find(|a| story.images.contains_key(&format!("{prefix}_{group}_{a}")))
                .map(|s| s.as_str())
                .unwrap_or(default);
            names.push(format!("{prefix}_{group}_{attr}"));
        }
        if prefix == "side_peri" {
            names.push(story.accessories[0][state.acc[0]].clone());
            names.push(story.accessories[2][state.acc[2]].clone());
        }
        return names;
    }
    let name = std::iter::once(sprite.tag.as_str())
        .chain(sprite.attrs.iter().map(|s| s.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    if story.images.contains_key(&name) {
        vec![name]
    } else {
        vec![]
    }
}
fn sprite_rect(sprite: &Sprite, t: &Texture2D) -> Rect {
    let w = t.width();
    let h = t.height();
    if sprite.tag == "bg" {
        return Rect::new(
            0.,
            if sprite.position == "top" {
                0.
            } else {
                720. - h
            },
            1280.,
            h * 1280. / w,
        );
    }
    if sprite.tag == "fg" {
        return Rect::new(0., 720. - h, w, h);
    }
    if sprite.tag == "cg" {
        if w >= 1200. {
            let scale = 1280. / w;
            return Rect::new(0., (720. - h * scale) * 0.5, 1280., h * scale);
        }
        return Rect::new(
            (1280. - w) * 0.5,
            if h < 500. { 80. } else { 720. - h },
            w,
            h,
        );
    }
    let x = if sprite.position.starts_with("left") {
        0.
    } else {
        (1280. - w) * 0.85
    };
    Rect::new(x, 720. - h, w, h)
}
async fn scene(art: &mut Art, state: &State, story: &Story, portrait: bool, lang: &str) {
    if art.textures.len() > 48 {
        let mut active = std::collections::HashSet::new();
        for s in &state.sprites {
            active.extend(layers(s, state, story));
        }
        active.extend(layers(
            &Sprite {
                tag: "side_peri".into(),
                attrs: state.attrs.clone(),
                position: String::new(),
            },
            state,
            story,
        ));
        for name in [
            "bg starfield",
            "bg chapterbreak",
            "titlelogo",
            "names1",
            "names2",
            "names3",
            "flower1",
            "petal",
            "petal2",
            "titlestar",
            "titleflower2",
            "shootingstar2",
        ] {
            active.insert(name.into());
        }
        for (current, previous, _) in art.face_history.values() {
            active.insert(current.clone());
            active.insert(previous.clone());
        }
        active.insert("titleflower".into());
        art.textures
            .retain(|name, _| active.contains(name) || name.starts_with("ui "));
    }
    art.stamps
        .retain(|tag, _| state.sprites.iter().any(|s| &s.tag == tag));
    for category in 0..5 {
        for s in &state.sprites {
            let cat = match s.tag.as_str() {
                "bg" | "black" | "white" => 0,
                "cg" => 2,
                "fg" => 3,
                "title" | "cinebars" | "lyrics" | "tld" | "cred1" | "cred2" | "cred6" | "cred7"
                | "credscroll" => 4,
                _ => 1,
            };
            if cat != category {
                continue;
            }
            let age = if art.animate { art.age(s) } else { 1000. };
            if s.tag == "white" {
                draw_rectangle(0., 0., 1280., 720., WHITE)
            }
            for name in layers(s, state, story) {
                art.ensure(&name, story).await;
                if let Some(t) = art.textures.get(&name) {
                    let mut r = sprite_rect(s, t);
                    let mut alpha = 1.;
                    match s.position.as_str() {
                        "righto" | "rightb" | "rightc" | "lefta" | "cg" | "cgspan" => {
                            alpha = (age / 0.5).min(1.);
                            let shift = (1. - alpha) * 30.;
                            if s.position == "rightb" {
                                r.x += shift
                            } else if s.position == "rightc" || s.position == "lefta" {
                                r.x -= shift
                            }
                        }
                        "slowpanup" => {
                            r.x = (1280. - r.w) * 0.66;
                            r.y = 720. - r.h + (age / 2.).min(1.) * 320.;
                            alpha = (age / 0.5).min(1.);
                        }
                        "slowpandown" => r.y = -(age / 12.).min(1.) * 560.,
                        "panup" => r.y = 720. - r.h + (age / 12.).min(1.) * 560.,
                        "bounce" | "bounce2" | "slowbounce" => {
                            r.y += if age < 0.4 {
                                (age * std::f32::consts::PI * 5.).sin().abs() * 12.
                            } else {
                                0.
                            }
                        }
                        _ => {}
                    }
                    if name.contains("_face_") {
                        art.blended_image(&name, r, alpha).await;
                    } else {
                        draw_texture_ex(
                            t,
                            r.x,
                            r.y,
                            Color::new(1., 1., 1., alpha),
                            DrawTextureParams {
                                dest_size: Some(vec2(r.w, r.h)),
                                flip_x: s.position == "flip",
                                ..Default::default()
                            },
                        );
                    }
                }
            }
            let key = std::iter::once(s.tag.as_str())
                .chain(s.attrs.iter().map(|s| s.as_str()))
                .collect::<Vec<_>>()
                .join(" ");
            if let Some(text) = story.text_images.get(&key) {
                art.centered(
                    &story.translate(text, lang),
                    if s.tag == "title" { 330. } else { 180. },
                    36,
                    if s.tag == "title" { INK } else { WHITE },
                );
            }
            if [
                "petals",
                "shootingstars",
                "firststar",
                "credflowers",
                "mmblossoms",
            ]
            .contains(&s.tag.as_str())
            {
                art.particles(&s.tag, story).await;
            }
            if s.tag == "cinebars" {
                draw_rectangle(0., 0., 1280., 60., BLACK);
                draw_rectangle(0., 660., 1280., 60., BLACK);
            }
            if s.tag == "credscroll" {
                let y = -30. - age / 66.8 * 2970.;
                for (i, name) in ["names1", "names2", "names3"].iter().enumerate() {
                    art.ensure(name, story).await;
                    if let Some(t) = art.textures.get(*name) {
                        art.image(name, 60., y + i as f32 * 750. + 50., t.width(), t.height());
                    }
                }
                for (i, name) in ["cred3", "cred4", "cred5"].iter().enumerate() {
                    if let Some(text) = story.text_images.get(*name) {
                        art.text(&clean_text(text), 60., y + i as f32 * 750. + 35., 30, WHITE);
                    }
                }
            }
            if s.tag == "lyrics" || s.tag == "tld" {
                let timeline = [
                    (0.7, "1a"),
                    (5.2, "1b"),
                    (9., "1c"),
                    (12.8, "1d"),
                    (17.7, "2a"),
                    (21.5, "2b"),
                    (26.4, "2c"),
                    (28.4, "2d"),
                    (33.2, "3a"),
                    (37.5, "3b"),
                    (40.9, "3c"),
                    (45., ""),
                    (77.3, "3a"),
                    (81.2, "4a"),
                    (83.9, "4b"),
                    (85., "4c"),
                    (86., "4d"),
                    (87., "3c"),
                ];
                if let Some((_, id)) = timeline.iter().rev().find(|(time, _)| age >= *time) {
                    let key = if s.tag == "tld" {
                        format!("lyric tl{id}")
                    } else {
                        format!("lyric {id}")
                    };
                    if let Some(text) = story.text_images.get(&key) {
                        art.centered(
                            &story.translate(text, lang),
                            if s.tag == "tld" { 40. } else { 700. },
                            28,
                            if s.tag == "tld" { BLUE } else { WHITE },
                        );
                    }
                }
            }
        }
    }
    if portrait && state.who == "w" {
        let p = Sprite {
            tag: "side_peri".into(),
            attrs: state.attrs.clone(),
            position: String::new(),
        };
        for name in layers(&p, state, story) {
            art.ensure(&name, story).await;
            art.blended_image(&name, Rect::new(0., 360., 360., 360.), 1.)
                .await;
        }
    }
}
fn mouse() -> Vec2 {
    let (x, y) = mouse_position();
    let scale = (screen_width() / 1280.).min(screen_height() / 720.);
    vec2(
        (x - (screen_width() - 1280. * scale) / 2.) / scale,
        (y - (screen_height() - 720. * scale) / 2.) / scale,
    )
}
fn button(art: &Art, label: &str, r: Rect) -> bool {
    let label = art.ui.get(label).map(String::as_str).unwrap_or(label);
    let hover = r.contains(mouse());
    draw_rectangle(
        r.x,
        r.y,
        r.w,
        r.h,
        if hover {
            Color::new(0.80, 0.88, 1., 0.97)
        } else {
            Color::new(0.96, 0.95, 1., 0.94)
        },
    );
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2., BLUE);
    let size = if label.chars().count() > 60 { 20 } else { 26 };
    let dim = measure_text(label, Some(&art.font), size, 1.);
    art.text(
        label,
        r.x + (r.w - dim.width) / 2.,
        r.y + r.h / 2. + dim.height / 2.,
        size,
        INK,
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
fn text_button(art: &Art, label: &str, r: Rect, title: bool) -> bool {
    let label = art.ui.get(label).map(String::as_str).unwrap_or(label);
    let hover = r.contains(mouse());
    let size = if title { 38 } else { 28 };
    let width = measure_text(label, Some(&art.font), size, 1.).width;
    let x = if title { r.x + (r.w - width) / 2. } else { r.x };
    let y = r.y + r.h / 2. + size as f32 * 0.35;
    if title {
        for (dx, dy) in [(-2., 0.), (2., 0.), (0., -2.), (0., 2.)] {
            art.text(
                label,
                x + dx,
                y + dy,
                size,
                if hover { WHITE } else { BLUE },
            );
        }
    }
    art.text(
        label,
        x,
        y,
        size,
        if title {
            if hover {
                BLUE
            } else {
                WHITE
            }
        } else if hover {
            BLUE
        } else {
            INK
        },
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
fn quick_button(art: &Art, label: &str, r: Rect, selected: bool) -> bool {
    let label = art.ui.get(label).map(String::as_str).unwrap_or(label);
    let hover = r.contains(mouse());
    art.text(
        label,
        r.x,
        r.y + 20.,
        18,
        if hover || selected { BLUE } else { INK },
    );
    if selected {
        draw_line(
            r.x,
            r.y + 24.,
            r.x + art.text_width(label, 18),
            r.y + 24.,
            2.,
            BLUE,
        );
    }
    hover && is_mouse_button_pressed(MouseButton::Left)
}
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Title,
    Game,
    Settings,
    Save,
    Load,
    History,
    Gallery,
    Language,
    Help,
    About,
}
fn sync_prefs(p: &mut Preferences, s: &State) {
    p.clear = s
        .vars
        .get("persistent.clear")
        .and_then(|v| v.as_bool())
        .unwrap_or(p.clear);
    p.fave = s.fave;
    for item in &s.seen {
        if !p.seen.contains(item) {
            p.seen.push(item.clone())
        }
    }
}
#[macroquad::main(conf)]
async fn main() {
    let exe = std::env::current_exe().unwrap();
    let mut candidates = vec![
        exe.parent().unwrap().to_path_buf(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ];
    if let Ok(dir) = std::env::current_dir() {
        candidates.push(dir)
    }
    if let Some(dir) = candidates.into_iter().find(|p| p.join("assets").is_dir()) {
        std::env::set_current_dir(dir).unwrap();
    }
    let story = Story::load();
    let saved_preferences: Option<serde_json::Value> = read_json("preferences.json");
    let legacy = saved_preferences
        .as_ref()
        .is_some_and(|value| value.get("version").is_none());
    let mut prefs: Preferences = saved_preferences
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    prefs.migrate(legacy);
    let font = load_ttf_font("assets/tl/None/Nunito-Bold.ttf")
        .await
        .expect("Missing font: keep assets next to exe");
    let mut art = Art {
        layouts: std::cell::RefCell::new(HashMap::new()),
        textures: HashMap::new(),
        font,
        inline_symbols: story.inline_symbols.clone(),
        sounds: HashMap::new(),
        music: String::new(),
        stamps: HashMap::new(),
        animate: true,
        font_lang: String::new(),
        face_history: HashMap::new(),
        ui_lang: String::new(),
        ui: HashMap::new(),
        fonts: HashMap::new(),
    };
    for file in [
        "None/Nunito-Bold.ttf",
        "ru/VDS_New.ttf",
        "zh/ResourceHanRoundedCN-Bold.ttf",
        "kor/Goyang.ttf",
        "thai/BoonJot-Regular.ttf",
    ] {
        if let Ok(font) = load_ttf_font(&format!("assets/tl/{file}")).await {
            art.fonts.insert(file.into(), font);
        }
    }
    for name in [
        "bg starfield",
        "bg chapterbreak",
        "titlelogo",
        "titleflower",
        "ui main_menu",
        "ui game_menu",
        "ui textbox",
        "ui namebox",
        "ui nvl",
    ] {
        art.ensure(name, &story).await;
    }
    for symbol in story.inline_symbols.values() {
        art.ensure(&symbol.image, &story).await;
    }
    for name in [
        "ui slider/horizontal_idle_bar",
        "ui slider/horizontal_idle_bar2",
        "ui slider/horizontal_hover_bar",
        "ui slider/horizontal_hover_bar2",
        "ui button/radio_foreground",
        "ui button/radio_selected_foreground",
        "ui button/check_foreground",
        "ui button/check_selected_foreground",
    ] {
        art.ensure(name, &story).await;
    }
    let mut state = State::default();
    let mut stop = Stop::End;
    let mut page = Page::Title;
    let mut return_page = Page::Title;
    let mut history: Vec<State> = vec![];
    let mut pause_until = 0.;
    let mut auto = false;
    let mut next_auto = 0.;
    let mut notification = String::new();
    let mut notice_until = 0.;
    let mut gallery_index = 0usize;
    let mut history_offset = 0usize;
    let mut about_index = 0usize;
    let mut fullscreen = prefs.fullscreen;
    if fullscreen {
        set_fullscreen(true);
    }
    let mut effect_id = 0;
    let mut effect_started = 0.;
    let mut text_pc = usize::MAX;
    let mut text_elapsed = 0.;
    let mut reveal_all = false;
    let mut skipping = false;
    let mut hide_dialogue = false;
    let smoke_mode = std::env::args()
        .find_map(|s| s.strip_prefix("--smoke=").map(str::to_owned))
        .unwrap_or_else(|| "dialogue".into());
    let smoke = std::env::args().any(|s| s.starts_with("--smoke"));
    if std::env::args().any(|s| s == "--audit-layout") {
        let mut checked = 0;
        for lang in ["", "es"] {
            art.font_for(lang).await;
            for label in [
                "start", "ch0", "ch4b", "ch7b", "ex1", "ex2", "ex3", "ex4", "ex5",
            ] {
                let mut sample = State::default();
                sample.start(&story, label);
                loop {
                    match sample.run(&story) {
                        Stop::Menu => sample.pc = story.ops[sample.pc - 1].choices[1].target,
                        Stop::End => break,
                        Stop::Say => {
                            if sample.who == "centered" {
                                continue;
                            }
                            let (text, width, height) = if sample.nvl_mode {
                                (
                                    sample
                                        .nvl
                                        .iter()
                                        .map(|s| clean_text(&story.translate(s, lang)))
                                        .collect::<Vec<_>>()
                                        .join("\n\n"),
                                    780.,
                                    610.,
                                )
                            } else {
                                (story.translate(&sample.text, lang), 692., 300.)
                            };
                            let (size, lines) = art.fitted(&text, width, height);
                            assert!(
                                lines.len() as f32 * size as f32 * 1.35 <= height,
                                "Vertical overflow: {lang} / {label} / {}",
                                sample.pc
                            );
                            for line in lines {
                                assert!(
                                    measure_text(&line.text, Some(&art.font), size, 1.).width
                                        <= width + 0.1,
                                    "Horizontal overflow: {lang} / {label}"
                                );
                            }
                            checked += 1;
                            if checked % 100 == 0 {
                                next_frame().await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        std::fs::write("layout-audit.txt",format!("Passed {checked} dialogue and narration layouts using the original fonts in English and Spanish. No horizontal or vertical overflow.\n")).unwrap();
        return;
    }
    let mut frames = 0;
    if smoke {
        state.start(&story, "start");
        for _ in 0..100 {
            stop = state.run(&story);
            if stop == Stop::Menu {
                state.pc = story.ops[state.pc - 1].choices[1].target;
            }
            if state.who == "p" {
                break;
            }
        }
        page = Page::Game;
        match smoke_mode.as_str() {
            "title" => page = Page::Title,
            "settings" | "settings-en" | "settings-es" => {
                if smoke_mode == "settings-en" {
                    prefs.lang.clear();
                } else if smoke_mode == "settings-es" {
                    prefs.lang = "es".into();
                }
                page = Page::Settings;
            }
            "language" => page = Page::Language,
            "about" => page = Page::About,
            "about-translations" => {
                page = Page::About;
                about_index = 1;
            }
            "about-patrons" => {
                page = Page::About;
                about_index = 7;
            }
            "shake" => {
                while state.effect != "vpunch" {
                    stop = state.run(&story);
                }
                effect_id = state.effect_id;
                effect_started = get_time();
            }
            "dress" => {
                stop = Stop::Dress;
                state.acc = [8, 10, 2];
            }
            "portrait" => {
                state.start(&story, "ch4");
                stop = state.run(&story);
                while state.who != "w" {
                    stop = state.run(&story);
                }
            }
            "heart" | "heart-red" | "heart-brown" => {
                let c = match smoke_mode.as_str() {
                    "heart-red" => '\u{2764}',
                    "heart-brown" => '\u{1f90e}',
                    _ => '\u{1f499}',
                };
                let op = story
                    .ops
                    .iter()
                    .find(|op| op.op == "say" && op.text.contains(c))
                    .unwrap();
                state.who = op.who.clone();
                state.text = op.text.clone();
                state.attrs = op.attrs.clone();
                state.nvl_mode = false;
            }
            "gallery" => {
                prefs.clear = true;
                page = Page::Gallery;
            }
            "history" => {
                let mut entries: Vec<_> = story
                    .ops
                    .iter()
                    .filter(|op| op.op == "say" && !op.who.is_empty() && op.who != "centered")
                    .collect();
                entries.sort_by_key(|op| {
                    std::cmp::Reverse(story.translate(&op.text, &prefs.lang).chars().count())
                });
                history = entries
                    .iter()
                    .take(5)
                    .map(|op| State {
                        who: op.who.clone(),
                        text: op.text.clone(),
                        ..Default::default()
                    })
                    .collect();
                page = Page::History;
                return_page = Page::Game;
            }
            "credits" => {
                state.start(&story, "credits");
                stop = state.run(&story);
                pause_until = get_time() + 2.;
            }
            _ => {}
        }
    }
    loop {
        art.font_for(&prefs.lang).await;
        if art.ui_lang != prefs.lang {
            art.ui = story
                .translations
                .get(&prefs.lang)
                .cloned()
                .unwrap_or_default();
            art.ui_lang = prefs.lang.clone();
        }
        if state.pc != text_pc {
            text_pc = state.pc;
            text_elapsed = 0.;
            reveal_all = false;
        }
        if page == Page::Game && stop == Stop::Say {
            text_elapsed += get_frame_time() as f64;
        }
        if smoke {
            reveal_all = true;
        }
        if page == Page::Game && is_key_pressed(KeyCode::Tab) {
            skipping = !skipping;
            if skipping {
                auto = false;
            }
        }
        if page == Page::Game && is_key_pressed(KeyCode::H) {
            hide_dialogue = !hide_dialogue;
        }
        if state.effect_id != effect_id {
            effect_id = state.effect_id;
            effect_started = get_time();
        }
        if fullscreen != prefs.fullscreen {
            fullscreen = prefs.fullscreen;
            set_fullscreen(fullscreen);
        }
        if skipping && stop == Stop::Say && !prefs.can_skip(state.pc) {
            skipping = false;
        }
        let skip_active = !smoke
            && (skipping || is_key_down(KeyCode::LeftControl))
            && (matches!(stop, Stop::Pause(_)) || prefs.can_skip(state.pc));
        art.animate = !(skip_active && prefs.skip_transitions);
        clear_background(BLACK);
        let scale = (screen_width() / 1280.).min(screen_height() / 720.);
        let camera = Camera2D {
            target: vec2(640., 360.),
            zoom: vec2(2. / 1280., 2. / 720.),
            viewport: Some((
                ((screen_width() - 1280. * scale) / 2.) as i32,
                ((screen_height() - 720. * scale) / 2.) as i32,
                (1280. * scale) as i32,
                (720. * scale) as i32,
            )),
            ..Default::default()
        };
        set_camera(&camera);
        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            prefs.fullscreen = fullscreen;
            set_fullscreen(fullscreen);
        }
        if page == Page::Title {
            state.music = "romance".into();
            art.image("bg starfield", 0., -560., 1280., 1280.);
            art.particles("mmblossoms", &story).await;
            art.image("titlelogo", 280., 72., 720., 360.);

            for (i, label) in ["Start", "Continue", "Options", "Quit"].iter().enumerate() {
                if i > 0 {
                    art.image("titleflower", 210. + i as f32 * 235. - 48., 543., 38., 38.);
                }
                if text_button(
                    &art,
                    &story.translate(label, &prefs.lang),
                    Rect::new(190. + i as f32 * 235., 530., 210., 65.),
                    true,
                ) {
                    match i {
                        0 => {
                            state = State {
                                fave: prefs.fave,
                                ..Default::default()
                            };
                            state
                                .vars
                                .insert("persistent.clear".into(), prefs.clear.into());
                            state.start(&story, "start");
                            history.clear();
                            stop = state.run(&story);
                            pause_until = get_time();
                            page = Page::Game;
                            state.sound = "start".into();
                        }
                        1 => {
                            return_page = page;
                            page = Page::Load;
                        }
                        2 => {
                            return_page = page;
                            page = Page::Settings;
                        }
                        _ => {
                            let _ = write_json("preferences.json", &prefs);
                            return;
                        }
                    }
                }
            }
            if prefs.clear
                && text_button(
                    &art,
                    &story.translate("Extras", &prefs.lang),
                    Rect::new(490., 615., 300., 55.),
                    true,
                )
            {
                state.start(&story, "ex_navi");
                stop = state.run(&story);
                page = Page::Game;
            }
        } else if page == Page::Game {
            let shake = if art.animate {
                punch_offset(&state.effect, get_time() - effect_started)
            } else {
                Vec2::ZERO
            };
            let mut shaken_camera = Camera2D {
                target: vec2(640. - shake.x, 360. - shake.y),
                zoom: camera.zoom,
                viewport: camera.viewport,
                ..Default::default()
            };
            set_camera(&shaken_camera);
            scene(&mut art, &state, &story, false, &prefs.lang).await;
            shaken_camera.target = camera.target;
            set_camera(&shaken_camera);
            let translated = story.translate(&state.text, &prefs.lang);
            let shown = visible_text(
                &translated,
                text_elapsed,
                prefs.text_cps,
                reveal_all || skip_active,
            );
            let text_complete = shown == clean_text(&translated);
            // A displayed line is read even when the player has not advanced yet.
            // Persist immediately so closing the window preserves skip eligibility.
            if !smoke && stop == Stop::Say && text_complete && prefs.mark_read(state.pc) {
                let _ = write_json("preferences.json", &prefs);
            }
            if !hide_dialogue && state.nvl_mode && stop == Stop::Say {
                art.image("ui nvl", 0., 0., 1280., 720.);
                let paragraphs: Vec<String> = state
                    .nvl
                    .iter()
                    .map(|text| clean_text(&story.translate(text, &prefs.lang)))
                    .collect();
                let full = paragraphs.join("\n\n");
                let prefix = if paragraphs.len() > 1 {
                    paragraphs[..paragraphs.len() - 1]
                        .join("\n\n")
                        .chars()
                        .count()
                        + 2
                } else {
                    0
                };
                let (size, lines) = art.fitted(&full, 780., 610.);
                art.flow(&lines, prefix + shown.chars().count(), 240., 48., size, INK);
            } else if !hide_dialogue && stop == Stop::Say && state.who != "centered" {
                let (size, lines) = art.fitted(&translated, 692., 300.);
                let body_height = lines.len() as f32 * size as f32 * 1.35;
                let box_height = (34. + body_height + 50.).max(200.);
                let box_top = 720. - box_height;
                art.image("ui textbox", 0., box_top + 15., 1280., box_height - 15.);
                if state.who == "w" {
                    let portrait = Sprite {
                        tag: "side_peri".into(),
                        attrs: state.attrs.clone(),
                        position: String::new(),
                    };
                    for name in layers(&portrait, &state, &story) {
                        art.ensure(&name, &story).await;
                        art.blended_image(&name, Rect::new(0., 360., 360., 360.), 1.)
                            .await;
                    }
                }
                let who = character_name(&state.who);
                if !who.is_empty() {
                    let metrics = measure_text(who, Some(&art.font), 38, 1.);
                    let name_top = box_top - 30.;
                    art.image(
                        "ui namebox",
                        340.,
                        name_top - 8.,
                        metrics.width + 26.,
                        metrics.height + 16.,
                    );
                    art.speaker(&state.who, who, 345., name_top);
                }
                let color = if state.who == "n" {
                    Color::from_rgba(121, 119, 152, 255)
                } else {
                    INK
                };
                art.flow(
                    &lines,
                    shown.chars().count(),
                    380.,
                    box_top + 34.,
                    size,
                    color,
                );
            }
            let mut advance = false;
            if stop == Stop::Say
                && state.text.contains("{nw}")
                && text_elapsed >= preferences::wait_seconds(&state.text)
            {
                advance = true;
            }
            if stop == Stop::Menu {
                let op = &story.ops[state.pc - 1];
                if !op.text.is_empty() {
                    art.wrap(
                        &story.translate(&op.text, &prefs.lang),
                        260.,
                        150.,
                        800.,
                        30,
                    );
                }
                for (i, c) in op.choices.iter().enumerate() {
                    if button(
                        &art,
                        &clean_text(&story.translate(&c.text, &prefs.lang)),
                        Rect::new(210., 200. + i as f32 * 64., 860., 53.),
                    ) {
                        state.pc = c.target;
                        if !prefs.skip_after_choices {
                            skipping = false;
                            auto = false;
                        }
                        advance = true;
                    }
                }
            }
            if stop == Stop::Dress {
                art.image("bg chapterbreak", 0., 0., 1280., 720.);
                art.text("Accessorize!", 120., 95., 40, INK);
                let p = Sprite {
                    tag: "side_peri".into(),
                    attrs: vec![state.outfit.clone(), "smile".into()],
                    position: String::new(),
                };
                for name in layers(&p, &state, &story) {
                    art.ensure(&name, &story).await;
                    art.image(&name, 100., 170., 440., 440.);
                }
                for group in 0..3 {
                    let y = 210. + group as f32 * 105.;
                    art.text(
                        &format!(
                            "Group {}: {} / {}",
                            group + 1,
                            state.acc[group],
                            story.accessories[group].len() - 1
                        ),
                        700.,
                        y,
                        28,
                        INK,
                    );
                    if button(&art, "<", Rect::new(610., y + 15., 75., 50.)) {
                        state.acc[group] = (state.acc[group] + story.accessories[group].len() - 1)
                            % story.accessories[group].len();
                    }
                    if button(&art, ">", Rect::new(1070., y + 15., 75., 50.)) {
                        state.acc[group] = (state.acc[group] + 1) % story.accessories[group].len();
                    }
                }
                if button(&art, "Save Favorite", Rect::new(610., 550., 250., 55.)) {
                    state.fave = state.acc;
                    prefs.fave = state.fave;
                }
                if button(&art, "Load Favorite", Rect::new(890., 550., 250., 55.)) {
                    state.acc = state.fave;
                }
                if button(&art, "Done", Rect::new(790., 625., 250., 55.)) {
                    advance = true;
                }
            }
            if stop == Stop::Pause(0.)
                || matches!(stop, Stop::Pause(_)) && get_time() >= pause_until
            {
                advance = true;
            }
            let click = is_mouse_button_pressed(MouseButton::Left) && mouse().y < 670.;
            let manual = !smoke
                && (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) || click);
            if manual {
                auto = false;
                skipping = false;
            }
            if matches!(stop, Stop::Say | Stop::Pause(_))
                && (manual || skip_active || (auto && text_complete && get_time() > next_auto))
            {
                if hide_dialogue {
                    hide_dialogue = false;
                } else if stop == Stop::Say && !text_complete && !skip_active {
                    reveal_all = true;
                } else {
                    advance = true;
                }
            }
            let side_rollback = click
                && ((prefs.rollback_side == "left" && mouse().x < 256.)
                    || (prefs.rollback_side == "right" && mouse().x > 1024.));
            if is_key_pressed(KeyCode::Left)
                || is_key_pressed(KeyCode::Backspace)
                || mouse_wheel().1 > 0.
                || side_rollback
            {
                if let Some(previous) = history.pop() {
                    state = previous;
                    stop = Stop::Say;
                    effect_id = state.effect_id;
                    advance = false;
                }
            }
            if !hide_dialogue
                && state
                    .vars
                    .get("quick_menu")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
            {
                for (i, label) in ["Back", "Skip", "Auto", "Menu", "Hide Textbox"]
                    .iter()
                    .enumerate()
                {
                    if quick_button(
                        &art,
                        label,
                        Rect::new(
                            770. + i as f32 * 95.,
                            684.,
                            if i == 4 { 130. } else { 85. },
                            30.,
                        ),
                        (i == 1 && skipping) || (i == 2 && auto),
                    ) {
                        match i {
                            0 => {
                                if let Some(previous) = history.pop() {
                                    state = previous;
                                    effect_id = state.effect_id;
                                    stop = Stop::Say;
                                    advance = false;
                                }
                            }
                            1 => {
                                skipping = !skipping;
                                if skipping {
                                    auto = false;
                                }
                            }
                            2 => {
                                auto = !auto;
                                if auto {
                                    skipping = false;
                                }
                                next_auto =
                                    get_time() + prefs.auto_delay(translated.chars().count());
                            }
                            3 => {
                                return_page = page;
                                page = Page::History;
                                history_offset = 0;
                            }
                            _ => hide_dialogue = true,
                        }
                    }
                }
            }
            if advance && page == Page::Game {
                if stop == Stop::Say {
                    if prefs.mark_read(state.pc) && !smoke {
                        let _ = write_json("preferences.json", &prefs);
                    }
                    history.push(state.clone());
                    if history.len() > 250 {
                        history.remove(0);
                    }
                }
                stop = state.run(&story);
                next_auto = get_time()
                    + prefs.auto_delay(story.translate(&state.text, &prefs.lang).chars().count());
                if let Stop::Pause(seconds) = stop {
                    pause_until = get_time() + seconds;
                }
                if stop == Stop::Gallery {
                    page = Page::Gallery;
                    return_page = Page::Game;
                    gallery_index = 0;
                }
                if stop == Stop::End {
                    sync_prefs(&mut prefs, &state);
                    let _ = write_json("preferences.json", &prefs);
                    page = Page::Title;
                    state.music.clear();
                }
            }
            if is_key_pressed(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                return_page = Page::Game;
                page = Page::Settings;
            }
        } else {
            art.image("bg chapterbreak", 0., 0., 1280., 720.);
            if matches!(
                page,
                Page::Settings
                    | Page::Language
                    | Page::Help
                    | Page::About
                    | Page::Save
                    | Page::Load
                    | Page::History
            ) {
                art.image("ui game_menu", 0., 0., 1280., 720.);
                let navigation = if return_page == Page::Title {
                    vec![
                        ("Start", Some(Page::Game)),
                        ("Load", Some(Page::Load)),
                        ("Options", Some(Page::Settings)),
                        ("About", Some(Page::About)),
                        ("Help", Some(Page::Help)),
                        ("Quit", None),
                    ]
                } else {
                    vec![
                        ("History", Some(Page::History)),
                        ("Save", Some(Page::Save)),
                        ("Load", Some(Page::Load)),
                        ("Options", Some(Page::Settings)),
                        ("Main Menu", Some(Page::Title)),
                        ("About", Some(Page::About)),
                        ("Help", Some(Page::Help)),
                        ("Quit", None),
                    ]
                };
                for (i, (label, destination)) in navigation.iter().enumerate() {
                    if text_button(
                        &art,
                        &story.translate(label, &prefs.lang),
                        Rect::new(80., 150. + i as f32 * 57., 230., 48.),
                        false,
                    ) {
                        let Some(destination) = destination else {
                            sync_prefs(&mut prefs, &state);
                            let _ = write_json("preferences.json", &prefs);
                            return;
                        };
                        if *destination == Page::Game {
                            state = State {
                                fave: prefs.fave,
                                ..Default::default()
                            };
                            state
                                .vars
                                .insert("persistent.clear".into(), prefs.clear.into());
                            state.start(&story, "start");
                            history.clear();
                            stop = state.run(&story);
                            pause_until = get_time();
                        }
                        if *destination == Page::Title {
                            sync_prefs(&mut prefs, &state);
                            let _ = write_json("preferences.json", &prefs);
                        }
                        page = *destination;
                    }
                }
            }
            if page == Page::Save || page == Page::Load {
                art.text(
                    if page == Page::Save { "Save" } else { "Load" },
                    100.,
                    85.,
                    40,
                    INK,
                );
                for slot in 1..=6 {
                    let name = format!("slot{slot}.json");
                    let saved: Option<State> = read_json(&name);
                    let preview = saved
                        .as_ref()
                        .map(|s| {
                            clean_text(&story.translate(&s.text, &prefs.lang))
                                .chars()
                                .take(60)
                                .collect::<String>()
                        })
                        .unwrap_or_else(|| "Empty Slot".into());
                    if button(
                        &art,
                        &format!("{slot}: {preview}"),
                        Rect::new(370., 115. + (slot - 1) as f32 * 75., 815., 60.),
                    ) {
                        if page == Page::Save {
                            sync_prefs(&mut prefs, &state);
                            notification = match write_json(&name, &state) {
                                Ok(()) => "Game saved".into(),
                                Err(e) => format!("Error: {e}"),
                            };
                            notice_until = get_time() + 3.;
                            let _ = write_json("preferences.json", &prefs);
                        } else if let Some(saved) = saved {
                            state = saved;
                            let op = &story.ops[state.pc - 1];
                            stop = match op.op.as_str() {
                                "menu" => Stop::Menu,
                                "dress" => Stop::Dress,
                                "pause" => Stop::Pause(0.),
                                _ => Stop::Say,
                            };
                            history.clear();
                            page = Page::Game;
                            pause_until = get_time();
                        }
                    }
                }
            } else if page == Page::Settings {
                if preferences::settings(&art, &story, &mut prefs) {
                    page = Page::Language;
                }
            } else if page == Page::Language {
                if preferences::languages(&art, &story, &mut prefs) {
                    page = Page::Settings;
                }
            } else if page == Page::Help {
                art.text(&story.translate("Help", &prefs.lang), 60., 70., 40, BLUE);
                art.wrap("Enter / Space / Click: advance dialogue.\nCtrl: skip read text. Tab: toggle skipping.\nLeft / Backspace / Mouse wheel up: rollback.\nEscape / Right click: game menu. F11: fullscreen.\nH: hide dialogue.\nOptions: text speed, auto-forward and audio.",370.,165.,800.,28);
            } else if page == Page::About {
                art.text(&story.translate("About", &prefs.lang), 60., 70., 40, BLUE);
                let count = story.credits.len();
                if count > 0 {
                    if button(&art, "Previous", Rect::new(370., 610., 240., 50.))
                        || is_key_pressed(KeyCode::Left)
                    {
                        about_index = (about_index + count - 1) % count;
                    }
                    if button(&art, "Next", Rect::new(910., 610., 240., 50.))
                        || is_key_pressed(KeyCode::Right)
                    {
                        about_index = (about_index + 1) % count;
                    }
                    let credit = &story.credits[about_index % count];
                    art.label(
                        &story.translate(&credit.heading, &prefs.lang),
                        370.,
                        145.,
                        790.,
                        32,
                        BLUE,
                    );
                    art.wrap(&credit.body, 370., 198., 790., 26);
                    art.text(
                        &format!("{} / {}", about_index + 1, count),
                        710.,
                        625.,
                        24,
                        INK,
                    );
                }
            } else if page == Page::History {
                art.text(&story.translate("History", &prefs.lang), 60., 70., 40, BLUE);
                let start = history.len().saturating_sub(5 + history_offset);
                let end = (start + 5).min(history.len());
                let entries: Vec<_> = history[start..end]
                    .iter()
                    .map(|entry| (entry, story.translate(&entry.text, &prefs.lang)))
                    .collect();
                let size = (12..=23)
                    .rev()
                    .find(|size| {
                        entries
                            .iter()
                            .map(|(_, text)| {
                                (art.lines(text, 600., *size).len() as f32 * *size as f32 * 1.35)
                                    .max(32.)
                                    + 20.
                            })
                            .sum::<f32>()
                            <= 460.
                    })
                    .unwrap_or(12);
                let mut top = 115.;
                for (entry, text) in entries {
                    let lines = art.lines(&text, 600., size);
                    art.speaker_sized(&entry.who, character_name(&entry.who), 370., top, 24);
                    art.flow(&lines, usize::MAX, 590., top, size, INK);
                    top += (lines.len() as f32 * size as f32 * 1.35).max(32.) + 20.;
                }
                if button(&art, "Previous", Rect::new(380., 590., 250., 55.)) {
                    history_offset = (history_offset + 5).min(history.len().saturating_sub(5));
                }
                if button(&art, "Next", Rect::new(700., 590., 250., 55.)) {
                    history_offset = history_offset.saturating_sub(5);
                }
            } else if page == Page::Gallery {
                let mut images: Vec<String> = prefs
                    .seen
                    .iter()
                    .chain(state.seen.iter())
                    .cloned()
                    .collect();
                if prefs.clear {
                    images.extend(
                        story
                            .images
                            .keys()
                            .filter(|name| name.starts_with("cg ") || name.starts_with("ex_"))
                            .cloned(),
                    );
                }
                images.sort();
                images.dedup();
                if let Some(name) = images.get(gallery_index.min(images.len().saturating_sub(1))) {
                    art.ensure(name, &story).await;
                    if let Some(t) = art.textures.get(name) {
                        let scale = (1100. / t.width()).min(550. / t.height());
                        let (w, h) = (t.width() * scale, t.height() * scale);
                        art.image(name, (1280. - w) / 2., 30., w, h);
                    }
                    art.text(
                        &format!("{} / {}", gallery_index + 1, images.len()),
                        570.,
                        620.,
                        28,
                        INK,
                    );
                } else {
                    art.text("Images unlock as you read the story.", 260., 320., 30, INK);
                }
                if button(&art, "<", Rect::new(100., 600., 130., 55.)) {
                    gallery_index = gallery_index.saturating_sub(1);
                }
                if button(&art, ">", Rect::new(1050., 600., 130., 55.)) {
                    gallery_index = (gallery_index + 1).min(images.len().saturating_sub(1));
                }
                for (i, name) in [
                    "date", "pastille", "peri", "astra", "party", "romance", "finale", "credits",
                ]
                .iter()
                .enumerate()
                {
                    if button(
                        &art,
                        &format!("#{}", i + 1),
                        Rect::new(240. + i as f32 * 95., 660., 85., 40.),
                    ) {
                        state.music = name.to_string();
                    }
                }
            }
            if text_button(
                &art,
                &story.translate("Return", &prefs.lang),
                Rect::new(20., 665., 220., 40.),
                true,
            ) || is_key_pressed(KeyCode::Escape)
            {
                let _ = write_json("preferences.json", &prefs);
                page = if page == Page::Language {
                    Page::Settings
                } else {
                    return_page
                };
                if page == Page::Game && stop == Stop::Gallery {
                    stop = state.run(&story);
                }
            }
        }
        if get_time() < notice_until {
            draw_rectangle(220., 15., 840., 48., WHITE);
            art.text(&notification, 245., 48., 23, INK);
        }
        art.audio(&mut state, &prefs).await;
        set_default_camera();
        frames += 1;
        if smoke && smoke_mode == "shake" && frames == 1 {
            get_screen_data().export_png("smoke-shake-motion.png");
        }
        if smoke && frames >= 30 {
            get_screen_data().export_png(&format!("smoke-{smoke_mode}.png"));
            return;
        }
        next_frame().await;
    }
}
