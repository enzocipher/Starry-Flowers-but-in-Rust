use crate::{mouse, Art, Story, BLUE, INK};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub version: u32,
    pub lang: String,
    pub volume: f32,
    pub sound_volume: f32,
    pub mute: bool,
    pub fullscreen: bool,
    pub rollback_side: String,
    pub skip_unseen: bool,
    pub skip_after_choices: bool,
    pub skip_transitions: bool,
    pub text_cps: f32,
    pub afm_time: f32,
    pub read: Vec<usize>,
    pub clear: bool,
    pub seen: Vec<String>,
    pub fave: [usize; 3],
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 2,
            lang: String::new(),
            volume: 1.,
            sound_volume: 1.,
            mute: false,
            fullscreen: false,
            rollback_side: "disable".into(),
            skip_unseen: false,
            skip_after_choices: false,
            skip_transitions: false,
            text_cps: 60.,
            afm_time: 15.,
            read: vec![],
            clear: false,
            seen: vec![],
            fave: [0; 3],
        }
    }
}
impl Preferences {
    pub fn migrate(&mut self, legacy: bool) {
        if legacy {
            self.version = 2;
            self.lang.clear();
            self.sound_volume = self.volume;
        }
    }
    pub fn auto_delay(&self, len: usize) -> f64 {
        self.afm_time.max(0.) as f64 * (25. + len as f64) / 250.
    }
    pub fn mark_read(&mut self, pc: usize) -> bool {
        if self.read.contains(&pc) {
            return false;
        }
        self.read.push(pc);
        true
    }
    pub fn can_skip(&self, pc: usize) -> bool {
        self.skip_unseen || self.read.contains(&pc)
    }
}
pub fn visible_text(text: &str, elapsed: f64, cps: f32, all: bool) -> String {
    let clean = crate::engine::clean_text(text);
    if all || cps <= 0. {
        return clean;
    }
    clean
        .chars()
        .take((elapsed * cps as f64) as usize)
        .collect()
}
pub fn wait_seconds(text: &str) -> f64 {
    text.split("{w=")
        .skip(1)
        .filter_map(|part| part.split('}').next()?.parse::<f64>().ok())
        .sum()
}
// Ren'Py's original Move offsets, 0.10-second bounce repeated for 0.275 seconds.
pub fn punch_offset(effect: &str, elapsed: f64) -> Vec2 {
    if !(0.0..0.275).contains(&elapsed) {
        return Vec2::ZERO;
    }
    let phase = (elapsed / 0.1) % 2.;
    let n = if phase <= 1. {
        1. - 2. * phase
    } else {
        -1. + 2. * (phase - 1.)
    };
    match effect {
        "vpunch" => vec2(0., n as f32 * 10.),
        "hpunch" => vec2(n as f32 * 15., 0.),
        _ => Vec2::ZERO,
    }
}
fn slider(art: &Art, value: &mut f32, min: f32, max: f32, r: Rect) {
    draw_rectangle(r.x, r.y + 12., r.w, 8., Color::new(0.77, 0.81, 0.91, 1.));
    let fraction = (*value - min) / (max - min);
    draw_rectangle(r.x, r.y + 12., r.w * fraction, 8., BLUE);
    draw_circle(r.x + r.w * fraction, r.y + 16., 11., BLUE);
    if r.contains(mouse()) && is_mouse_button_down(MouseButton::Left) {
        *value = min + (max - min) * ((mouse().x - r.x) / r.w).clamp(0., 1.);
    }
    let _ = art;
}
fn option(art: &Art, text: &str, r: Rect, selected: bool) -> bool {
    let hover = r.contains(mouse());
    if selected {
        draw_circle(r.x + 8., r.y + r.h / 2., 6., BLUE);
    } else {
        draw_circle_lines(
            r.x + 8.,
            r.y + r.h / 2.,
            6.,
            1.5,
            Color::new(0.65, 0.69, 0.76, 1.),
        );
    }
    art.text(
        text,
        r.x + 24.,
        r.y + r.h / 2. + 9.,
        26,
        if hover || selected { BLUE } else { INK },
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}
pub fn settings(art: &Art, story: &Story, p: &mut Preferences) -> bool {
    let tr = |s: &str| story.translate(s, &p.lang);
    art.text(&tr("Options"), 60., 70., 40, BLUE);
    let mut language = false;
    for (x, title) in [(370., "Display"), (650., "Rollback Side"), (930., "Skip")] {
        art.text(&tr(title), x, 160., 30, BLUE);
    }
    for (i, (id, label)) in [(false, "Window"), (true, "Fullscreen")].iter().enumerate() {
        if option(
            art,
            &tr(label),
            Rect::new(370., 185. + i as f32 * 43., 245., 38.),
            p.fullscreen == *id,
        ) {
            p.fullscreen = *id;
        }
    }
    language |= option(
        art,
        &tr("Language..."),
        Rect::new(370., 271., 245., 38.),
        false,
    );
    for (i, (id, label)) in [("disable", "Disable"), ("left", "Left"), ("right", "Right")]
        .iter()
        .enumerate()
    {
        if option(
            art,
            &tr(label),
            Rect::new(650., 185. + i as f32 * 43., 245., 38.),
            p.rollback_side == *id,
        ) {
            p.rollback_side = id.to_string();
        }
    }
    if option(
        art,
        &tr("Unseen Text"),
        Rect::new(930., 185., 280., 38.),
        p.skip_unseen,
    ) {
        p.skip_unseen = !p.skip_unseen;
    }
    if option(
        art,
        &tr("After Choices"),
        Rect::new(930., 228., 280., 38.),
        p.skip_after_choices,
    ) {
        p.skip_after_choices = !p.skip_after_choices;
    }
    if option(
        art,
        &tr("Transitions"),
        Rect::new(930., 271., 280., 38.),
        p.skip_transitions,
    ) {
        p.skip_transitions = !p.skip_transitions;
    }
    art.text(&tr("Text Speed"), 370., 390., 30, BLUE);
    let mut text_speed = if p.text_cps == 0. { 201. } else { p.text_cps };
    slider(
        art,
        &mut text_speed,
        1.,
        201.,
        Rect::new(370., 410., 345., 38.),
    );
    p.text_cps = if text_speed >= 200.5 { 0. } else { text_speed };
    art.text(if p.text_cps == 0. { "∞" } else { "" }, 730., 438., 25, INK);
    art.text(&tr("Auto-Forward Time"), 370., 495., 30, BLUE);
    slider(
        art,
        &mut p.afm_time,
        0.,
        30.,
        Rect::new(370., 515., 345., 38.),
    );
    art.text(&tr("Music Volume"), 820., 390., 30, BLUE);
    slider(art, &mut p.volume, 0., 1., Rect::new(820., 410., 350., 38.));
    art.text(&tr("Sound Volume"), 820., 495., 30, BLUE);
    slider(
        art,
        &mut p.sound_volume,
        0.,
        1.,
        Rect::new(820., 515., 350., 38.),
    );
    if option(
        art,
        &tr("Mute All"),
        Rect::new(820., 585., 280., 38.),
        p.mute,
    ) {
        p.mute = !p.mute;
    }
    language
}
pub fn languages(art: &Art, story: &Story, p: &mut Preferences) -> bool {
    art.text(&story.translate("Language", &p.lang), 60., 70., 40, BLUE);
    for (i, (id, label)) in [
        ("", "English"),
        ("es", "Español"),
        ("fr", "Français"),
        ("ger", "Deutsch"),
        ("ita", "Italiano"),
        ("pl", "Polski"),
        ("ptbr", "Português"),
        ("ru", "Русский"),
        ("ukr", "Українська"),
        ("tur", "Türkçe"),
        ("zh", "简体中文"),
        ("kor", "한국어"),
        ("vi", "Tiếng Việt"),
        ("thai", "ภาษาไทย"),
    ]
    .iter()
    .enumerate()
    {
        let r = Rect::new(
            360. + (i % 3) as f32 * 290.,
            130. + (i / 3) as f32 * 90.,
            265.,
            65.,
        );
        let file = match *id {
            "ru" | "ukr" => "ru/VDS_New.ttf",
            "zh" => "zh/ResourceHanRoundedCN-Bold.ttf",
            "kor" => "kor/Goyang.ttf",
            "thai" => "thai/BoonJot-Regular.ttf",
            _ => "None/Nunito-Bold.ttf",
        };
        let font = art.fonts.get(file).unwrap_or(&art.font);
        let hover = r.contains(mouse());
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            if hover {
                Color::new(0.80, 0.88, 1., 1.)
            } else {
                Color::new(0.95, 0.95, 1., 1.)
            },
        );
        let width = measure_text(label, Some(font), 26, 1.).width;
        draw_text_ex(
            label,
            r.x + (r.w - width) / 2.,
            r.y + 42.,
            TextParams {
                font: Some(font),
                font_size: 26,
                color: INK,
                ..Default::default()
            },
        );
        if hover && is_mouse_button_pressed(MouseButton::Left) {
            p.lang = id.to_string();
            return true;
        }
    }
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_original() {
        let p = Preferences::default();
        assert_eq!(p.lang, "");
        assert_eq!(p.text_cps, 60.);
        assert_eq!(p.afm_time, 15.);
        assert!(!p.skip_unseen);
    }
    #[test]
    fn legacy_migration_preserves_progress() {
        let mut p = Preferences {
            lang: "es".into(),
            clear: true,
            volume: 0.4,
            ..Default::default()
        };
        p.migrate(true);
        assert_eq!(p.lang, "");
        assert!(p.clear);
        assert_eq!(p.sound_volume, 0.4);
    }
    #[test]
    fn punches_match_original_timing() {
        assert_eq!(punch_offset("vpunch", 0.), vec2(0., 10.));
        assert_eq!(punch_offset("vpunch", 0.1), vec2(0., -10.));
        assert_eq!(punch_offset("hpunch", 0.2), vec2(15., 0.));
        assert_eq!(punch_offset("vpunch", 0.275), Vec2::ZERO);
    }
    #[test]
    fn text_reveal_is_unicode_safe() {
        assert_eq!(visible_text("{i}¡Hola!{/i}", 0.5, 4., false), "¡H");
        assert_eq!(visible_text("Hello", 0., 0., false), "Hello");
    }
    #[test]
    fn skip_and_auto_preferences_have_effect() {
        let p = Preferences {
            read: vec![42],
            ..Default::default()
        };
        assert!(p.can_skip(42));
        assert!(!p.can_skip(43));
        assert!(p.auto_delay(250) > p.auto_delay(10));
        assert_eq!(p.auto_delay(75), 6.);
    }
    #[test]
    fn read_dialogue_survives_restart_and_stops_at_unread() {
        let story = crate::engine::Story::load();
        let mut state = crate::engine::State::default();
        state.start(&story, "start");
        while state.run(&story) != crate::engine::Stop::Say {}
        let previous = state.clone();
        let mut p = Preferences::default();
        assert!(!p.can_skip(state.pc));
        assert!(p.mark_read(state.pc));
        assert!(!p.mark_read(state.pc));
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(restored.can_skip(previous.pc));
        while state.run(&story) != crate::engine::Stop::Say {}
        assert!(!restored.can_skip(state.pc));
    }
    #[test]
    fn zero_auto_delay_advances_after_reveal() {
        let p = Preferences {
            afm_time: 0.,
            ..Default::default()
        };
        assert_eq!(p.auto_delay(80), 0.);
        assert_ne!(visible_text("Hello", 0., 60., false), "Hello");
        assert_eq!(visible_text("Hello", 1., 60., false), "Hello");
    }
}
