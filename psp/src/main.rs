#![no_std]
#![no_main]
extern crate alloc;
use alloc::{
    collections::VecDeque,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use psp::sys::{self, CtrlButtons as B};
use serde::{Deserialize, Serialize};
psp::module!("StarryFlowers", 1, 0);
mod engine {
    include!(concat!(env!("OUT_DIR"), "/engine.rs"));
}
mod layout {
    include!(concat!(env!("OUT_DIR"), "/layout.rs"));
}
mod audio;
mod audit;
mod io;
mod playback;
use audio::Audio;
mod gpu;
mod render;
use engine::{clean_text, Sprite, State, Stop, Story};
use render::{Renderer, BLUE, H, INK, W, WHITE};
const EXTRAS: [&str; 8] = ["ch0", "ch4b", "ch7b", "ex1", "ex2", "ex3", "ex4", "ex5"];
const EXTRA_TITLES: [&str; 8] = [
    "Prologue",
    "Chapter 4.5 - Stay",
    "Chapter 7.5 - \"I love you\"",
    "Busted Lamp",
    "Atelier Sweets",
    "Cuddly Witch",
    "Old Photos",
    "Night on the Town",
];
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Prefs {
    lang: String,
    cps: f32,
    delay: f64,
    music: f32,
    sound: f32,
    mute: bool,
    unseen: bool,
    after_choices: bool,
    transitions: bool,
    read: Vec<usize>,
    clear: bool,
    fave: [usize; 3],
}
impl Default for Prefs {
    fn default() -> Self {
        Self {
            lang: String::new(),
            cps: 60.,
            delay: 15.,
            music: 0.8,
            sound: 1.,
            mute: false,
            unseen: false,
            after_choices: false,
            transitions: false,
            read: vec![],
            clear: false,
            fave: [0; 3],
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Title,
    About,
    Game,
    Menu,
    Settings,
    Save,
    Load,
    Gallery,
    Extras,
}
fn now() -> f64 {
    unsafe { sys::sceKernelGetSystemTimeWide() as f64 / 1_000_000. }
}
fn load<T: serde::de::DeserializeOwned>(name: &str) -> Option<T> {
    let path = format!("ms0:/PSP/SAVEDATA/STARRYFLOWERS/{name}.JSON");
    io::read(&path)
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .or_else(|| {
            io::read(format!("{path}.BAK"))
                .ok()
                .and_then(|data| serde_json::from_slice(&data).ok())
        })
}
fn save<T: Serialize>(name: &str, value: &T) -> bool {
    let dir = io::save_dir();
    let Ok(data) = serde_json::to_vec(value) else {
        return false;
    };
    let path = format!("{dir}/{name}.JSON");
    let temp = format!("{path}.TMP");
    io::write(&temp, &data).is_ok() && io::replace(&temp, &path)
}
fn screenshot(r: &mut Renderer, name: &str) {
    let mut bmp = vec![0u8; 54 + W * H * 4];
    bmp[..2].copy_from_slice(b"BM");
    let len = bmp.len() as u32;
    bmp[2..6].copy_from_slice(&len.to_le_bytes());
    bmp[10..14].copy_from_slice(&54u32.to_le_bytes());
    bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
    bmp[18..22].copy_from_slice(&(W as i32).to_le_bytes());
    bmp[22..26].copy_from_slice(&(-(H as i32)).to_le_bytes());
    bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
    bmp[28..30].copy_from_slice(&32u16.to_le_bytes());
    for (i, p) in r.pixels().iter().enumerate() {
        bmp[54 + i * 4..58 + i * 4].copy_from_slice(&[
            (*p >> 16) as u8,
            (*p >> 8) as u8,
            *p as u8,
            255,
        ]);
    }
    let _ = io::write(&format!("{}/{name}.BMP", io::save_dir()), &bmp);
}
fn menu(r: &mut Renderer, title: &str, labels: &[String], selected: usize) {
    r.frame.fill(WHITE);
    r.image("ui game_menu", 0, 0, Some((W, H)), 255, false);
    r.frame.scene_len = r.frame.draws.len();
    r.text(title, 22, 12, 18, BLUE);
    for (i, label) in labels.iter().enumerate() {
        let y = 45 + i as i32 * 23;
        if i == selected {
            r.rect(105, y, 360, 23, 0x55ffa358);
        }
        r.text(label, 115, y, 12, if i == selected { BLUE } else { INK });
    }
    r.text("D-pad: select   X: confirm   O: back", 18, 253, 10, INK);
}
fn about_scene(r: &mut Renderer, story: &Story, lang: &str, index: usize) {
    r.frame.fill(WHITE);
    r.image("ui game_menu", 0, 0, Some((W, H)), 255, false);
    r.frame.scene_len = r.frame.draws.len();
    r.frame.prefix_sealed = true;
    r.text(&story.translate("About", lang), 22, 12, 18, BLUE);
    if let Some(credit) = story.credits.get(index) {
        r.label(
            &story.translate(&credit.heading, lang),
            130,
            43,
            315.,
            14,
            BLUE,
        );
        r.flow(
            &credit.body.replace("RENPY-LICENSE.txt", "RENPY.TXT"),
            130,
            69,
            315.,
            170,
            usize::MAX,
            INK,
        );
    }
    r.text(
        &format!(
            "{} / {}   Left/Right: page   X: next   O: back",
            index + 1,
            story.credits.len()
        ),
        110,
        251,
        10,
        INK,
    );
}
fn settings_scene(r: &mut Renderer, story: &Story, p: &Prefs, selected: usize) {
    r.frame.fill(WHITE);
    r.image("ui game_menu", 0, 0, Some((W, H)), 255, false);
    // Cache only the static background; controls remain responsive overlays.
    r.frame.scene_len = r.frame.draws.len();
    r.frame.prefix_sealed = true;
    let tr = |s: &str| story.translate(s, &p.lang);
    r.text(&tr("Options"), 22, 12, 18, BLUE);
    r.text(&tr("Display"), 110, 39, 12, BLUE);
    r.text(&tr("Skip"), 292, 39, 12, BLUE);
    if selected == 0 {
        r.rect(106, 59, 166, 19, 0x33ffa358);
    }
    r.image(
        "ui button/radio_selected_foreground",
        110,
        61,
        Some((10, 14)),
        255,
        false,
    );
    r.text(
        &format!(
            "{}: {}",
            tr("Language"),
            if p.lang.is_empty() {
                "English"
            } else {
                "Español"
            }
        ),
        125,
        61,
        12,
        if selected == 0 { BLUE } else { INK },
    );
    for (index, label, on, y) in [
        (5, "Unseen Text", p.unseen, 61),
        (6, "After Choices", p.after_choices, 82),
        (7, "Transitions", p.transitions, 103),
        (8, "Mute All", p.mute, 221),
    ] {
        if selected == index {
            r.rect(288, y - 2, 170, 19, 0x33ffa358);
        }
        r.image(
            if on {
                "ui button/check_selected_foreground"
            } else {
                "ui button/check_foreground"
            },
            292,
            y,
            Some((10, 14)),
            255,
            false,
        );
        r.label(
            &tr(label),
            307,
            y,
            150.,
            12,
            if selected == index { BLUE } else { INK },
        );
    }
    for (index, label, value, fraction, x, y) in [
        (
            1,
            "Text Speed",
            if p.cps == 0. {
                "∞".into()
            } else {
                format!("{:.0}", p.cps)
            },
            if p.cps == 0. { 1. } else { p.cps / 200. },
            110,
            133,
        ),
        (
            2,
            "Auto-Forward Time",
            format!("{:.0}", p.delay),
            p.delay as f32 / 30.,
            110,
            177,
        ),
        (
            3,
            "Music Volume",
            format!("{:.0}%", p.music * 100.),
            p.music,
            292,
            133,
        ),
        (
            4,
            "Sound Volume",
            format!("{:.0}%", p.sound * 100.),
            p.sound,
            292,
            177,
        ),
    ] {
        if selected == index {
            r.rect(x - 4, y - 2, 170, 36, 0x22ffa358);
        }
        r.label(
            &tr(label),
            x,
            y,
            148. - r.width(&value, 10),
            12,
            if selected == index { BLUE } else { INK },
        );
        r.text(&value, x + 155 - r.width(&value, 10) as i32, y, 10, INK);
        r.image(
            "ui slider/horizontal_idle_bar",
            x,
            y + 16,
            Some((156, 16)),
            255,
            false,
        );
        r.image_progress(
            if selected == index {
                "ui slider/horizontal_hover_bar2"
            } else {
                "ui slider/horizontal_idle_bar2"
            },
            x as f32,
            (y + 16) as f32,
            156.,
            16.,
            fraction,
        );
    }
    if selected == 9 {
        r.rect(106, 219, 166, 19, 0x33ffa358);
    }
    r.text(
        &tr("Return"),
        110,
        221,
        12,
        if selected == 9 { BLUE } else { INK },
    );
    r.text(
        "D-pad: select   Left/Right: adjust   X: toggle   O: back",
        18,
        253,
        10,
        INK,
    );
}
fn dress_scene(r: &mut Renderer, state: &State, story: &Story) {
    r.frame.fill(WHITE);
    r.image("bg chapterbreak", 0, 0, Some((W, H)), 255, false);
    let sprite = Sprite {
        tag: "side_peri".into(),
        attrs: vec![state.outfit.clone(), "smile".into()],
        position: String::new(),
    };
    for name in render::layers(&sprite, state, story) {
        r.image(&name, 22, 45, Some((195, 195)), 255, false);
    }
    r.frame.scene_len = r.frame.draws.len();
    r.frame.prefix_sealed = true;
}
fn psp_main() {
    unsafe {
        psp::init_heap(18 * 1024 * 1024);
    }
    psp::enable_home_button();
    unsafe {
        sys::sceDisplaySetMode(sys::DisplayMode::Lcd, W, H);
        sys::sceCtrlSetSamplingCycle(0);
        sys::sceCtrlSetSamplingMode(sys::CtrlMode::Digital);
    }
    let root = if io::exists("disc0:/PSP_GAME/USRDIR/DATA/MANIFEST.JSON") {
        "disc0:/PSP_GAME/USRDIR"
    } else {
        "."
    };
    let audit = io::exists(&format!("{root}/DATA/AUDIT"));
    let mut r = Renderer::new(root);
    let story = Story::load();
    let mut p: Prefs = load("PREFS").unwrap_or_default();
    let (music, sound) = (
        Audio::new(root, &r.manifest.audio, true),
        Audio::new(root, &r.manifest.audio, false),
    );
    let mut state = State::default();
    let mut stop = Stop::End;
    let mut page = Page::Title;
    let mut return_page = Page::Title;
    let mut selected = 0usize;
    let mut history: VecDeque<State> = VecDeque::new();
    let (mut auto, mut skipping, mut hidden) = (false, false, false);
    let mut previous = B::empty();
    let mut pad: sys::SceCtrlData = unsafe { core::mem::zeroed() };
    let mut pc = usize::MAX;
    let mut started = now();
    let mut pause_until = 0.;
    let mut skip_gate = playback::SkipGate::default();
    let mut prefs_dirty = false;
    let mut prefs_saved_at = now();
    let mut seen_effect_id = state.effect_id;
    let mut effect_started = now();
    let mut all = false;
    let mut base = gpu::Frame::new();
    let mut old = base.clone();
    let mut buffer = 0;
    let mut frames = 0;
    let mut notice = String::new();
    let mut notice_until = 0.;
    let mut slot_labels: Vec<String> = Vec::new();
    let mut slots_page: Option<Page> = None;
    loop {
        let time = now();
        unsafe {
            sys::sceCtrlPeekBufferPositive(&mut pad, 1);
        }
        let pressed = pad.buttons & !previous;
        previous = pad.buttons;
        let cross = pressed.contains(B::CROSS);
        let circle = pressed.contains(B::CIRCLE);
        let mut advance = false;
        if state.pc != pc {
            pc = state.pc;
            started = time;
            all = false;
            old.clone_from(&base);
            r.scene(&state, &story);
            base.clone_from(&r.frame);
            selected = 0;
            if let Stop::Pause(s) = stop {
                pause_until = time + s;
            }
        }
        if state.effect_id != seen_effect_id {
            seen_effect_id = state.effect_id;
            effect_started = time;
        }
        if audit && frames == 30 {
            screenshot(&mut r, "PSP-TITLE");
            state.start(&story, "start");
            stop = state.run(&story);
            page = Page::Game;
        }
        if audit && frames == 90 && io::exists(&format!("{root}/DATA/PERF")) {
            let navigation = audit::performance(&mut r, &story);
            let skip = audit::skip_playback(&mut r, &story);
            let result = serde_json::json!({"navigation": navigation, "skip_playback": skip});
            save("PERF", &result);
            unsafe {
                sys::sceKernelExitGame();
            }
            return;
        }
        if audit && frames == 90 {
            screenshot(&mut r, "PSP-DIALOGUE");
            let mut result = audit::run(&mut r, &story);
            result["audio_blocks"] = music.blocks().into();
            assert!(music.blocks() > 0, "Audio did not play");
            save("AUDIT", &result);
            unsafe {
                sys::sceKernelExitGame();
            }
            return;
        }
        match page {
            Page::Title => {
                state.music = "romance".into();
                r.frame.fill(WHITE);
                if p.clear {
                    r.image("bg starfield", 0, -210, Some((480, 480)), 255, false);
                } else {
                    r.image("ui main_menu", 0, 0, Some((W, H)), 255, false);
                }
                r.image("titlelogo", 105, 27, Some((270, 135)), 255, false);
                r.frame.scene_len = r.frame.draws.len();
                r.frame.prefix_sealed = true;
                for i in 0..if p.clear { 0 } else { 20 } {
                    let i = i as f32;
                    let x = ((i * 151. + time as f32 * 6.) % 1400. - 60.) * 0.375;
                    let y = ((i * 97. + time as f32 * 18.) % 820. - 80.) * 0.375;
                    r.image(
                        if i as usize % 2 == 0 {
                            "titlestar"
                        } else {
                            "titleflower2"
                        },
                        x as i32,
                        y as i32,
                        Some((15, 15)),
                        128,
                        false,
                    );
                }
                let labels = [
                    "Start", "Continue", "Settings", "Extras", "Gallery", "About",
                ];
                if pressed.contains(B::LEFT) {
                    selected = (selected + labels.len() - 1) % labels.len();
                }
                if pressed.contains(B::RIGHT) {
                    selected = (selected + 1) % labels.len();
                }
                for (i, label) in labels.iter().enumerate() {
                    r.label(
                        label,
                        10 + i as i32 * 78,
                        220,
                        76.,
                        12,
                        if selected == i { BLUE } else { INK },
                    );
                }
                r.text("D-pad: select   X: confirm", 135, 250, 10, INK);
                if cross {
                    match selected {
                        0 => {
                            state = State {
                                fave: p.fave,
                                ..Default::default()
                            };
                            state.vars.insert("persistent.clear".into(), p.clear.into());
                            state.start(&story, "start");
                            stop = state.run(&story);
                            history.clear();
                            page = Page::Game;
                            pc = usize::MAX;
                            auto = false;
                            skipping = false;
                        }
                        1 => {
                            return_page = Page::Title;
                            page = Page::Load;
                        }
                        2 => {
                            return_page = Page::Title;
                            page = Page::Settings;
                        }
                        3 if p.clear => {
                            return_page = Page::Title;
                            page = Page::Extras;
                        }
                        4 if p.clear => {
                            return_page = Page::Title;
                            page = Page::Gallery;
                        }
                        5 => {
                            return_page = Page::Title;
                            page = Page::About;
                        }
                        _ => {
                            notice = "Finish the story to unlock extras.".into();
                            notice_until = time + 2.;
                        }
                    }
                    selected = 0;
                }
            }
            Page::About => {
                let count = story.credits.len();
                if count > 0 {
                    if pressed.contains(B::LEFT) {
                        selected = (selected + count - 1) % count;
                    }
                    if pressed.contains(B::RIGHT) || cross {
                        selected = (selected + 1) % count;
                    }
                    about_scene(&mut r, &story, &p.lang, selected);
                }
                if circle {
                    page = return_page;
                    selected = 0;
                }
            }
            Page::Game => {
                if pressed.contains(B::SQUARE) {
                    auto = !auto;
                    skipping = false;
                }
                if pressed.contains(B::SELECT) {
                    auto = false;
                }
                if pressed.contains(B::TRIANGLE) {
                    hidden = !hidden;
                }
                if pressed.contains(B::START) {
                    page = Page::Menu;
                    return_page = Page::Game;
                    selected = 0;
                    auto = false;
                    skipping = false;
                }
                if circle || pressed.contains(B::LTRIGGER) {
                    if let Some(prev) = history.pop_back() {
                        state = prev;
                        stop = Stop::Say;
                        pc = state.pc;
                        started = time;
                        all = true;
                        r.scene(&state, &story);
                        base.clone_from(&r.frame);
                        seen_effect_id = state.effect_id;
                        effect_started = time - 1.;
                        auto = false;
                        skipping = false;
                    }
                }

                let text = clean_text(&story.translate(&state.text, &p.lang));
                let total = text.chars().count();
                let eligible = p.unseen || p.read.contains(&state.pc);
                let interrupt = (cross && matches!(stop, Stop::Say | Stop::Pause(_)))
                    || circle
                    || pressed.intersects(B::LTRIGGER | B::START | B::SQUARE);
                let skip = skip_gate.update(
                    &mut skipping,
                    pad.buttons.contains(B::RTRIGGER),
                    pressed.contains(B::SELECT),
                    interrupt,
                    stop != Stop::Say || eligible,
                ) && matches!(stop, Stop::Say | Stop::Pause(_));
                let skip_advance = skip_gate.advance_due(skip, time);
                let shown = if all || skip || p.cps <= 0. || audit {
                    total
                } else {
                    ((time - started) * p.cps as f64) as usize
                };
                let complete = shown >= total;
                if stop == Stop::Say && complete && !p.read.contains(&state.pc) {
                    p.read.push(state.pc);
                    prefs_dirty = true;
                }
                r.frame.clone_from(&base);
                let age = time - started;
                if !skip && !p.transitions && age < 0.18 {
                    r.frame.fade_from(&old, (age / 0.18 * 255.) as u32);
                }
                let effect_age = time - effect_started;
                if !skip
                    && !p.transitions
                    && ["vpunch", "hpunch"].contains(&state.effect.as_str())
                    && effect_age < 0.275
                {
                    let phase = (effect_age % 0.1) / 0.1;
                    let offset = if phase < 0.25 {
                        phase * 4.
                    } else if phase < 0.75 {
                        2. - phase * 4.
                    } else {
                        phase * 4. - 4.
                    };
                    let (dx, dy) = if state.effect == "vpunch" {
                        (0, (offset * 4.) as i32)
                    } else {
                        ((offset * 6.) as i32, 0)
                    };
                    r.frame.offset(dx, dy);
                }
                if stop == Stop::Say && !hidden {
                    r.dialogue(&state, &story, &p.lang, shown);
                }
                if matches!(stop, Stop::Say | Stop::Pause(_)) {
                    if cross {
                        auto = false;
                        skipping = false;
                        if hidden {
                            hidden = false;
                        } else if !complete && stop == Stop::Say {
                            all = true;
                        } else {
                            advance = true;
                        }
                    }
                    if skip_advance
                        || auto && complete && age > p.delay.max(0.) * (25. + total as f64) / 250.
                    {
                        advance = true;
                    }
                }
                if let Stop::Pause(_) = stop {
                    if time >= pause_until {
                        advance = true;
                    }
                }
                if stop == Stop::Menu {
                    let op = &story.ops[state.pc - 1];
                    let len = op.choices.len();
                    if pressed.contains(B::UP) {
                        selected = (selected + len - 1) % len;
                    }
                    if pressed.contains(B::DOWN) {
                        selected = (selected + 1) % len;
                    }
                    for (i, c) in op.choices.iter().enumerate() {
                        let y = 72 + i as i32 * 48;
                        r.rect(
                            50,
                            y,
                            380,
                            42,
                            if i == selected {
                                0xefffdbbe
                            } else {
                                0xefffffff
                            },
                        );
                        r.flow(
                            &story.translate(&c.text, &p.lang),
                            60,
                            y + 3,
                            355.,
                            40,
                            usize::MAX,
                            INK,
                        );
                    }
                    if cross {
                        state.pc = op.choices[selected].target;
                        if !p.after_choices {
                            auto = false;
                            let _ = skip_gate.update(
                                &mut skipping,
                                pad.buttons.contains(B::RTRIGGER),
                                false,
                                true,
                                true,
                            );
                        }
                        advance = true;
                    }
                }
                if stop == Stop::Dress {
                    dress_scene(&mut r, &state, &story);
                    if pressed.contains(B::UP) {
                        selected = (selected + 2) % 3;
                    }
                    if pressed.contains(B::DOWN) {
                        selected = (selected + 1) % 3;
                    }
                    let len = story.accessories[selected].len();
                    if pressed.contains(B::LEFT) {
                        state.acc[selected] = (state.acc[selected] + len - 1) % len;
                    }
                    if pressed.contains(B::RIGHT) {
                        state.acc[selected] = (state.acc[selected] + 1) % len;
                    }
                    for i in 0..3 {
                        r.text(
                            &format!(
                                "Group {}: {} / {}",
                                i + 1,
                                state.acc[i] + 1,
                                story.accessories[i].len()
                            ),
                            250,
                            72 + i as i32 * 36,
                            12,
                            if selected == i { BLUE } else { INK },
                        );
                    }
                    if pressed.contains(B::SQUARE) {
                        state.fave = state.acc;
                        p.fave = state.acc;
                        save("PREFS", &p);
                    }
                    if pressed.contains(B::TRIANGLE) {
                        state.acc = state.fave;
                    }
                    r.text("X: done   Square: favorite", 230, 207, 10, INK);
                    r.text("Triangle: load favorite", 230, 226, 10, INK);
                    if cross {
                        advance = true;
                    }
                }
                if stop != Stop::Dress {
                    r.rect(0, 254, 480, 18, 0xbfffffff);
                    r.text(
                        if auto {
                            "AUTO   X: stop"
                        } else if skipping {
                            "SKIP   X: stop"
                        } else {
                            "X: next   O: back   R: skip   Square: auto   Start: menu"
                        },
                        8,
                        256,
                        10,
                        INK,
                    );
                }
            }
            Page::Menu | Page::Settings | Page::Save | Page::Load | Page::Extras => {
                let labels: Vec<String> = match page {
                    Page::Menu => ["Return", "Save", "Load", "Settings", "About", "Main Menu"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect(),
                    Page::Settings => vec![
                        format!(
                            "Language: {}",
                            if p.lang.is_empty() {
                                "English"
                            } else {
                                "Español"
                            }
                        ),
                        format!("Text speed: {:.0}", p.cps),
                        format!("Auto time: {:.0}", p.delay),
                        format!("Music volume: {:.0}%", p.music * 100.),
                        format!("Sound volume: {:.0}%", p.sound * 100.),
                        format!("Skip unseen text: {}", p.unseen),
                        format!("Skip after choices: {}", p.after_choices),
                        format!("Skip transitions: {}", p.transitions),
                        format!("Mute All: {}", p.mute),
                        "Return".into(),
                    ],
                    Page::Extras => EXTRA_TITLES
                        .iter()
                        .map(|label| story.translate(label, &p.lang))
                        .collect(),
                    _ => {
                        if slots_page != Some(page) {
                            slot_labels = (1..=6)
                                .map(|slot| {
                                    format!(
                                        "Slot {slot}: {}",
                                        load::<State>(&format!("SLOT{slot}"))
                                            .map(|s| clean_text(&s.text)
                                                .chars()
                                                .take(35)
                                                .collect::<String>())
                                            .unwrap_or("Empty".into())
                                    )
                                })
                                .collect();
                            slots_page = Some(page);
                        }
                        slot_labels.clone()
                    }
                };
                let len = labels.len();
                if pressed.contains(B::UP) {
                    selected = (selected + len - 1) % len;
                }
                if pressed.contains(B::DOWN) {
                    selected = (selected + 1) % len;
                }
                if page == Page::Settings {
                    settings_scene(&mut r, &story, &p, selected);
                } else {
                    menu(
                        &mut r,
                        match page {
                            Page::Menu => "Menu",
                            Page::Settings => "Settings",
                            Page::Save => "Save",
                            Page::Load => "Load",
                            _ => "Extras",
                        },
                        &labels,
                        selected,
                    );
                }
                if circle {
                    page = if page == Page::Settings && return_page == Page::Game {
                        Page::Menu
                    } else {
                        return_page
                    };
                    selected = 0;
                } else if page == Page::Settings {
                    let direction = if pressed.contains(B::LEFT) {
                        -1.
                    } else if pressed.contains(B::RIGHT) || cross {
                        1.
                    } else {
                        0.
                    };
                    if direction != 0. {
                        match selected {
                            0 => {
                                p.lang = if p.lang.is_empty() {
                                    "es".into()
                                } else {
                                    String::new()
                                }
                            }
                            1 => p.cps = (p.cps + direction * 10.).clamp(0., 200.),
                            2 => p.delay = (p.delay + direction as f64).clamp(0., 30.),
                            3 => p.music = (p.music + direction * 0.1).clamp(0., 1.),
                            4 => p.sound = (p.sound + direction * 0.1).clamp(0., 1.),
                            5 => p.unseen = !p.unseen,
                            6 => p.after_choices = !p.after_choices,
                            7 => p.transitions = !p.transitions,
                            8 => p.mute = !p.mute,
                            _ => {
                                page = return_page;
                                selected = 0;
                            }
                        }
                        save("PREFS", &p);
                    }
                } else if cross {
                    match page {
                        Page::Menu => {
                            page = [
                                Page::Game,
                                Page::Save,
                                Page::Load,
                                Page::Settings,
                                Page::About,
                                Page::Title,
                            ][selected];
                            if page == Page::About {
                                return_page = Page::Menu;
                            }
                            if page == Page::Title {
                                save("PREFS", &p);
                            }
                            selected = 0;
                        }
                        Page::Save => {
                            slots_page = None;
                            notice = if save(&format!("SLOT{}", selected + 1), &state) {
                                "Game saved"
                            } else {
                                "Save failed"
                            }
                            .into();
                            notice_until = time + 2.;
                            save("PREFS", &p);
                        }
                        Page::Load => {
                            if let Some(s) = load::<State>(&format!("SLOT{}", selected + 1))
                                .filter(|s| s.pc > 0 && s.pc <= story.ops.len())
                            {
                                state = s;
                                stop = match story.ops[state.pc - 1].op.as_str() {
                                    "menu" => Stop::Menu,
                                    "dress" => Stop::Dress,
                                    "pause" => Stop::Pause(0.),
                                    _ => Stop::Say,
                                };
                                page = Page::Game;
                                history.clear();
                                pc = usize::MAX;
                                auto = false;
                                skipping = false;
                            }
                        }
                        Page::Extras => {
                            state = State::default();
                            state.start(&story, EXTRAS[selected]);
                            stop = state.run(&story);
                            page = Page::Game;
                            pc = usize::MAX;
                        }
                        _ => {}
                    }
                }
            }
            Page::Gallery => {
                let mut images: Vec<_> = story
                    .images
                    .keys()
                    .filter(|k| k.starts_with("cg "))
                    .cloned()
                    .collect();
                images.sort();
                if !images.is_empty() {
                    selected %= images.len();
                    r.frame.fill(0xff000000);
                    if let Some(info) = r.manifest.images.get(&images[selected]) {
                        let scale = (480.0 / info.original_width as f32)
                            .min(272.0 / info.original_height as f32);
                        let w = (info.original_width as f32 * scale + 0.5) as usize;
                        let h = (info.original_height as f32 * scale + 0.5) as usize;
                        r.image(
                            &images[selected],
                            (480 - w as i32) / 2,
                            (272 - h as i32) / 2,
                            Some((w, h)),
                            255,
                            false,
                        );
                        r.frame.scene_len = r.frame.draws.len();
                    }
                    r.rect(0, 250, 480, 22, 0xccffffff);
                    r.text(
                        &format!("{} / {}   {}", selected + 1, images.len(), images[selected]),
                        10,
                        252,
                        10,
                        INK,
                    );
                    if pressed.contains(B::LEFT) {
                        selected = (selected + images.len() - 1) % images.len();
                    }
                    if pressed.contains(B::RIGHT) {
                        selected = (selected + 1) % images.len();
                    }
                }
                if circle {
                    page = Page::Title;
                    selected = 0;
                }
                if stop == Stop::Gallery && cross {
                    page = Page::Game;
                    advance = true;
                }
            }
        }
        if advance && page == Page::Game {
            if stop == Stop::Say {
                if !p.read.contains(&state.pc) {
                    p.read.push(state.pc);
                    prefs_dirty = true;
                }
                history.push_back(state.clone());
                if history.len() > 32 {
                    history.pop_front();
                }
            }
            stop = state.run(&story);
            if stop == Stop::Gallery {
                page = Page::Gallery;
            }
            if stop == Stop::End {
                p.clear = state
                    .vars
                    .get("persistent.clear")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(p.clear);
                save("PREFS", &p);
                page = Page::Title;
                auto = false;
                skipping = false;
            }
        }
        if time < notice_until {
            r.rect(35, 5, 410, 25, WHITE);
            r.text(&notice, 42, 9, 12, INK);
        }
        music.update(
            r.manifest
                .audio
                .get(&state.music)
                .map(String::as_str)
                .unwrap_or(""),
            if p.mute { 0. } else { p.music },
            false,
        );
        sound.update(
            r.manifest
                .audio
                .get(&state.sound)
                .map(String::as_str)
                .unwrap_or(""),
            if p.mute { 0. } else { p.sound },
            !state.sound.is_empty(),
        );
        state.sound.clear();
        if page != Page::Save && page != Page::Load {
            slots_page = None;
        }
        if prefs_dirty && (page != Page::Game || time - prefs_saved_at >= 2.) {
            if save("PREFS", &p) {
                prefs_dirty = false;
                prefs_saved_at = time;
            }
        }
        r.present(buffer);
        buffer = 1 - buffer;
        frames += 1;
    }
}
