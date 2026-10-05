use crate::engine::{clean_text, Sprite, State, Story};
use alloc::collections::BTreeMap as HashMap;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use serde::Deserialize;

pub const W: usize = 480;
pub const H: usize = 272;
pub const INK: u32 = 0xff5c3638;
pub const WHITE: u32 = 0xff_ffffff;
pub const BLUE: u32 = 0xffffa358;

#[derive(Clone, Deserialize)]
pub struct ImageInfo {
    pub file: String,
    pub width: usize,
    pub height: usize,
    pub original_width: usize,
    pub original_height: usize,
}
#[derive(Deserialize)]
pub struct Glyph {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub advance: f32,
}
#[derive(Deserialize)]
pub struct Font {
    pub file: String,
    pub width: usize,
    pub height: usize,
    pub glyphs: HashMap<char, Glyph>,
    #[serde(skip)]
    pub data: Vec<u8>,
}
#[derive(Deserialize)]
pub struct Manifest {
    pub images: HashMap<String, ImageInfo>,
    pub fonts: HashMap<String, Font>,
    pub audio: HashMap<String, String>,
}
pub struct Texture {
    pub info: ImageInfo,
    pub data: Vec<u8>,
}
pub struct Renderer {
    pub frame: Vec<u32>,
    pub manifest: Manifest,
    textures: HashMap<String, Texture>,
    root: String,
}
pub fn blend(dst: u32, src: u32, alpha: u32) -> u32 {
    let a = ((src >> 24) * alpha) / 255;
    let mut out = 0xff000000;
    for shift in [0, 8, 16] {
        out |= ((((src >> shift) & 255) * a + ((dst >> shift) & 255) * (255 - a)) / 255) << shift;
    }
    out
}
fn blit(
    frame: &mut [u32],
    texture: &Texture,
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    alpha: u32,
    flip: bool,
) {
    if width == 0 || height == 0 {
        return;
    }
    for dy in 0..height {
        let py = y + dy as i32;
        if !(0..H as i32).contains(&py) {
            continue;
        }
        let sy = dy * texture.info.height / height;
        for dx in 0..width {
            let px = x + dx as i32;
            if !(0..W as i32).contains(&px) {
                continue;
            }
            let sx = if flip { width - 1 - dx } else { dx } * texture.info.width / width;
            let offset = (sy * texture.info.width + sx) * 4;
            let src = u32::from_le_bytes(texture.data[offset..offset + 4].try_into().unwrap());
            if src >> 24 == 0 {
                continue;
            }
            let index = py as usize * W + px as usize;
            frame[index] = if src >> 24 == 255 && alpha == 255 {
                src
            } else {
                blend(frame[index], src, alpha)
            };
        }
    }
}
impl Renderer {
    pub fn new(root: &str) -> Self {
        let mut manifest: Manifest = serde_json::from_slice(
            &crate::io::read(format!("{root}/DATA/MANIFEST.JSON")).expect("Missing PSP assets"),
        )
        .unwrap();
        for font in manifest.fonts.values_mut() {
            font.data = crate::io::read(format!("{root}/DATA/{}", font.file)).unwrap();
            assert_eq!(font.data.len(), font.width * font.height);
        }
        Self {
            frame: vec![WHITE; W * H],
            manifest,
            textures: HashMap::new(),
            root: root.into(),
        }
    }
    pub fn image(
        &mut self,
        name: &str,
        x: i32,
        y: i32,
        size: Option<(usize, usize)>,
        alpha: u32,
        flip: bool,
    ) {
        if !self.textures.contains_key(name) {
            let Some(info) = self.manifest.images.get(name).cloned() else {
                return;
            };
            let Ok(data) = crate::io::read(format!("{}/DATA/{}", self.root, info.file)) else {
                return;
            };
            if data.len() != info.width * info.height * 4 {
                return;
            }
            // Bound the software texture cache for PSP-1000's 32 MiB RAM.
            if self.textures.values().map(|t| t.data.len()).sum::<usize>() + data.len()
                > 4 * 1024 * 1024
            {
                self.textures.clear();
            }
            self.textures.insert(name.into(), Texture { info, data });
        }
        let texture = &self.textures[name];
        let (w, h) = size.unwrap_or((texture.info.width, texture.info.height));
        blit(&mut self.frame, texture, x, y, w, h, alpha, flip);
    }
    pub fn rect(&mut self, x: i32, y: i32, w: usize, h: usize, color: u32) {
        for py in y.max(0)..(y + h as i32).min(H as i32) {
            for px in x.max(0)..(x + w as i32).min(W as i32) {
                let i = py as usize * W + px as usize;
                self.frame[i] = blend(self.frame[i], color, 255);
            }
        }
    }
    pub fn width(&self, text: &str, size: usize) -> f32 {
        let font = &self.manifest.fonts[&size.to_string()];
        text.chars()
            .map(|c| {
                if c == '💙' {
                    size as f32 * 0.85
                } else {
                    font.glyphs
                        .get(&c)
                        .map(|g| g.advance)
                        .unwrap_or(size as f32 * 0.5)
                }
            })
            .sum()
    }
    pub fn text(&mut self, text: &str, x: i32, y: i32, size: usize, color: u32) {
        let font = &self.manifest.fonts[&size.to_string()];
        let mut cursor = x as f32;
        for c in text.chars() {
            if c == '💙' {
                let unit = size as f32 / 18.;
                for dy in 0..size {
                    for dx in 0..size {
                        let px = (dx as f32 / unit - 7.5) / 7.5;
                        let py = -(dy as f32 / unit - 8.) / 7.5;
                        if {
                            let a = px * px + py * py - 1.;
                            a * a * a - px * px * py * py * py
                        } <= 0.
                        {
                            let tx = cursor as i32 + dx as i32;
                            let ty = y + dy as i32;
                            if (0..W as i32).contains(&tx) && (0..H as i32).contains(&ty) {
                                self.frame[ty as usize * W + tx as usize] = 0xffffa358;
                            }
                        }
                    }
                }
                cursor += size as f32 * 0.85;
                continue;
            }
            let Some(g) = font.glyphs.get(&c) else {
                cursor += size as f32 * 0.5;
                continue;
            };
            for dy in 0..g.height {
                for dx in 0..g.width {
                    let tx = cursor as i32 + dx as i32;
                    let ty = y + dy as i32;
                    if !(0..W as i32).contains(&tx) || !(0..H as i32).contains(&ty) {
                        continue;
                    }
                    let alpha = font.data[(g.y + dy) * font.width + g.x + dx] as u32;
                    if alpha != 0 {
                        let i = ty as usize * W + tx as usize;
                        self.frame[i] = blend(self.frame[i], color, alpha);
                    }
                }
            }
            cursor += g.advance;
        }
    }
    pub fn name(&mut self, who: &str, text: &str, x: i32, y: i32) {
        let (outer, inner) = match who {
            "p" => (0xffeddaff, 0xff754db8),
            "w" => (0xffffdbbe, 0xffc18350),
            "a" => (0xffffdaf3, 0xff9858a2),
            "c" => (0xffdae3ff, 0xff5866a2),
            _ => (0xffffdbbe, 0xffc18350),
        };
        for (radius, color) in [(3, outer), (1, inner)] {
            for (dx, dy) in [
                (-radius, 0),
                (radius, 0),
                (0, -radius),
                (0, radius),
                (-radius, -radius),
                (radius, radius),
                (-radius, radius),
                (radius, -radius),
            ] {
                self.text(text, x + dx, y + dy, 14, color);
            }
        }
        self.text(text, x, y, 14, WHITE);
    }
    pub fn flow(
        &mut self,
        text: &str,
        x: i32,
        y: i32,
        width: f32,
        height: usize,
        revealed: usize,
        color: u32,
    ) -> usize {
        let text = clean_text(text);
        let (size, lines) = [14, 12, 10]
            .into_iter()
            .map(|size| {
                (
                    size,
                    crate::layout::wrap(&text, width, |s| self.width(s, size)),
                )
            })
            .find(|(s, l)| l.len() * (s * 14 / 10) <= height)
            .unwrap_or_else(|| (10, crate::layout::wrap(&text, width, |s| self.width(s, 10))));
        for (i, line) in lines.iter().enumerate() {
            let n = revealed
                .saturating_sub(line.start)
                .min(line.end - line.start);
            self.text(
                &line.text.chars().take(n).collect::<String>(),
                x,
                y + (i * size * 14 / 10) as i32,
                size,
                color,
            );
        }
        lines.len() * size * 14 / 10
    }
    pub fn scene(&mut self, state: &State, story: &Story) {
        self.frame.fill(0xff000000);
        for category in 0..5 {
            for s in &state.sprites {
                let cat = match s.tag.as_str() {
                    "bg" | "black" | "white" => 0,
                    "cg" => 2,
                    "fg" => 3,
                    "title" | "cinebars" | "lyrics" | "tld" | "cred1" | "cred2" | "cred6"
                    | "cred7" | "credscroll" => 4,
                    _ => 1,
                };
                if cat != category {
                    continue;
                }
                if s.tag == "white" {
                    self.frame.fill(WHITE);
                }
                for name in layers(s, state, story) {
                    let Some(info) = self.manifest.images.get(&name) else {
                        continue;
                    };
                    let (w, h) = (info.width, info.height);
                    let (x, y, size) = match s.tag.as_str() {
                        "bg" => (
                            0,
                            if s.position == "top" {
                                0
                            } else {
                                270 - h as i32
                            },
                            Some((480, h)),
                        ),
                        "cg" if info.original_width >= 1200 => {
                            (0, (270 - h as i32) / 2, Some((480, h)))
                        }
                        "cg" => (
                            (480 - w as i32) / 2,
                            if info.original_height < 500 {
                                30
                            } else {
                                270 - h as i32
                            },
                            None,
                        ),
                        "fg" => (0, 270 - h as i32, None),
                        _ => (
                            if s.position.starts_with("left") {
                                0
                            } else {
                                ((480 - w as i32) as f32 * 0.85) as i32
                            },
                            270 - h as i32,
                            None,
                        ),
                    };
                    self.image(&name, x, y, size, 255, s.position == "flip");
                }
                let key = core::iter::once(s.tag.as_str())
                    .chain(s.attrs.iter().map(|s| s.as_str()))
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Some(text) = story.text_images.get(&key) {
                    self.flow(
                        text,
                        45,
                        if s.tag == "title" { 110 } else { 55 },
                        390.,
                        160,
                        usize::MAX,
                        WHITE,
                    );
                }
                if s.tag == "cinebars" {
                    self.rect(0, 0, 480, 22, 0xff000000);
                    self.rect(0, 248, 480, 22, 0xff000000);
                }
            }
        }
    }
    pub fn dialogue(&mut self, state: &State, story: &Story, lang: &str, revealed: usize) {
        if state.nvl_mode {
            self.image("ui nvl", 0, 0, Some((480, 272)), 255, false);
            let paragraphs: Vec<_> = state
                .nvl
                .iter()
                .map(|s| clean_text(&story.translate(s, lang)))
                .collect();
            let prefix = paragraphs
                .iter()
                .take(paragraphs.len().saturating_sub(1))
                .map(|s| s.chars().count() + 2)
                .sum::<usize>();
            self.flow(
                &paragraphs.join("\n\n"),
                90,
                18,
                293.,
                230,
                prefix + revealed,
                INK,
            );
            return;
        }
        let text = story.translate(&state.text, lang);
        if state.who == "centered" {
            self.flow(&text, 48, 105, 384., 150, revealed, WHITE);
            return;
        }
        let lines = crate::layout::wrap(&clean_text(&text), 260., |s| self.width(s, 12));
        let height = (lines.len() * 17 + 36).clamp(75, 140);
        let top = 270 - height as i32;
        self.image(
            "ui textbox",
            0,
            top + 6,
            Some((480, height - 6)),
            255,
            false,
        );
        if state.who == "w" {
            let sprite = Sprite {
                tag: "side_peri".into(),
                attrs: state.attrs.clone(),
                position: String::new(),
            };
            for name in layers(&sprite, state, story) {
                self.image(&name, 0, 135, Some((135, 135)), 255, false);
            }
        }
        if !state.who.is_empty() && state.who != "n" {
            let name = match state.who.as_str() {
                "p" => "Pastille",
                "w" => "Periwinkle",
                "a" => "Astragalus",
                "c" => "Cassia",
                "j" => "Jam",
                "k" => "Kardaemon",
                "r" => "Amaretti",
                "u" => "Nasty Witch",
                "h" => "Himbo Witch",
                "g" => "Gumdrop",
                _ => "",
            };
            let name = story.translate(name, lang);
            self.name(&state.who, &name, 129, top - 17);
        }
        self.flow(
            &text,
            142,
            top + 10,
            260.,
            height - 27,
            revealed,
            if state.who == "n" { 0xff987779 } else { INK },
        );
    }
    pub fn present(&self, buffer: usize) {
        unsafe {
            let base = (psp::sys::sceGeEdramGetAddr() as usize | 0x40000000) as *mut u32;
            let base = base.add(buffer * 512 * H);
            for y in 0..H {
                core::ptr::copy_nonoverlapping(
                    self.frame.as_ptr().add(y * W),
                    base.add(y * 512),
                    W,
                );
            }
            psp::sys::sceDisplayWaitVblankStart();
            psp::sys::sceDisplaySetFrameBuf(
                base.cast(),
                512,
                psp::sys::DisplayPixelFormat::Psm8888,
                psp::sys::DisplaySetBufSync::Immediate,
            );
        }
    }
}
pub fn layers(sprite: &Sprite, state: &State, story: &Story) -> Vec<String> {
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
                .map(String::as_str)
                .unwrap_or(default);
            names.push(format!("{prefix}_{group}_{attr}"));
        }
        if prefix == "side_peri" {
            names.push(story.accessories[0][state.acc[0]].clone());
            names.push(story.accessories[2][state.acc[2]].clone());
        }
        names
    } else {
        let name = core::iter::once(sprite.tag.as_str())
            .chain(sprite.attrs.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        if story.images.contains_key(&name) {
            vec![name]
        } else {
            vec![]
        }
    }
}
