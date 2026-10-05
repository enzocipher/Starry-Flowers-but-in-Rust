//! Executes on the emulated PSP, including its allocator and Memory Stick I/O.
use crate::{
    engine::{State, Stop, Story},
    load,
    render::{Renderer, INK},
    save, screenshot, Prefs,
};
use alloc::{format, vec};

pub fn run(renderer: &mut Renderer, story: &Story) -> serde_json::Value {
    let mut lines = 0usize;
    let mut symbols_seen = vec![];
    let mut decisions = 0usize;
    let mut outfits = 0usize;
    let mut finishes = 0usize;
    for branch in 0..2 {
        let mut state = State::default();
        state.start(story, "start");
        let mut sample = false;
        for _ in 0..12000 {
            match state.run(story) {
                Stop::Say => {
                    lines += 1;
                    for (symbol, name) in [
                        ('\u{1f499}', "BLUE"),
                        ('\u{2764}', "RED"),
                        ('\u{1f90e}', "BROWN"),
                    ] {
                        if state.text.contains(symbol) && !symbols_seen.contains(&symbol) {
                            renderer.scene(&state, story);
                            renderer.dialogue(&state, story, "", usize::MAX);
                            screenshot(renderer, &format!("PSP-HEART-{name}"));
                            symbols_seen.push(symbol);
                        }
                    }
                    if !sample && state.who == "p" {
                        renderer.scene(&state, story);
                        renderer.dialogue(&state, story, "", usize::MAX);
                        screenshot(renderer, &format!("PSP-PASTILLE-{branch}"));
                        assert!(save("AUDIT-SLOT", &state));
                        let restored: State = load("AUDIT-SLOT").expect("Save reload failed");
                        assert_eq!(restored.pc, state.pc);
                        assert_eq!(restored.text, state.text);
                        sample = true;
                    }
                    if lines % 200 == 0 {
                        renderer.scene(&state, story);
                        renderer.dialogue(&state, story, "es", usize::MAX);
                        renderer.present(0);
                    }
                }
                Stop::Menu => {
                    decisions += 1;
                    let choices = &story.ops[state.pc - 1].choices;
                    state.pc = choices[branch.min(choices.len() - 1)].target;
                }
                Stop::Dress => {
                    outfits += 1;
                    state.acc = [branch; 3];
                    if outfits == 1 || outfits == 5 {
                        crate::dress_scene(renderer, &state, story);
                        screenshot(renderer, &format!("PSP-ACCESSORIES-{branch}"));
                    }
                }
                Stop::End => {
                    finishes += 1;
                    break;
                }
                _ => {}
            }
        }
    }
    assert_eq!(finishes, 2);
    assert_eq!(symbols_seen.len(), 3);
    let mut extras = 0usize;
    for extra in crate::EXTRAS {
        let mut state = State::default();
        state.start(story, extra);
        for _ in 0..2000 {
            match state.run(story) {
                Stop::Say => {
                    lines += 1;
                }
                Stop::Menu => {
                    state.pc = story.ops[state.pc - 1].choices[0].target;
                }
                Stop::End | Stop::Gallery => {
                    extras += 1;
                    break;
                }
                _ => {}
            }
        }
    }
    assert_eq!(extras, 8);
    let mut prefs = Prefs {
        read: vec![15, 22],
        ..Default::default()
    };
    assert!(save("AUDIT-PREFS", &prefs));
    prefs.lang = "es".into();
    prefs.mute = true;
    assert!(save("AUDIT-PREFS", &prefs));
    let restored: Prefs = load("AUDIT-PREFS").unwrap();
    assert_eq!(restored.lang, "es");
    assert!(restored.mute);
    assert_eq!(restored.read, prefs.read);
    crate::settings_scene(renderer, story, &Prefs::default(), 0);
    screenshot(renderer, "PSP-SETTINGS");
    let mut preview = Prefs::default();
    preview.lang = "es".into();
    preview.cps = 0.;
    preview.mute = true;
    crate::settings_scene(renderer, story, &preview, 8);
    screenshot(renderer, "PSP-SETTINGS-ES");
    let perf = performance(renderer, story);
    renderer.frame.fill(0xffffffff);
    renderer.text("PSP audit passed", 25, 25, 18, INK);
    renderer.text(
        &format!("{lines} lines; {finishes} endings; {extras} extras"),
        25,
        65,
        12,
        INK,
    );
    renderer.present(0);
    screenshot(renderer, "PSP-AUDIT-PASSED");
    serde_json::json!({"passed":true,"dialogues":lines,"endings":finishes,"extras":extras,"decisions":decisions,"accessory_screens":outfits,"save_reload":true,"settings_overwrite":true,"inline_symbols":symbols_seen.len(),"performance":perf})
}

pub fn performance(renderer: &mut Renderer, story: &Story) -> serde_json::Value {
    let mut preview = Prefs::default();
    preview.lang = "es".into();
    preview.mute = true;
    preview.cps = 0.;
    // Compare the retained scene with a direct draw, then exercise navigation.
    crate::settings_scene(renderer, story, &preview, 8);
    let cached = renderer.pixels();
    screenshot(renderer, "PSP-SETTINGS-CACHED");
    renderer.frame.scene_len = 0;
    let direct = renderer.pixels();
    screenshot(renderer, "PSP-SETTINGS-DIRECT");
    let differences = cached
        .iter()
        .zip(&direct)
        .filter(|(a, b)| {
            [0, 8, 16].iter().any(|shift| {
                (((**a >> shift) & 255) as i32 - ((**b >> shift) & 255) as i32).abs() > 3
            })
        })
        .count();
    let severe = cached
        .iter()
        .zip(&direct)
        .filter(|(a, b)| {
            [0, 8, 16].iter().any(|shift| {
                (((**a >> shift) & 255) as i32 - ((**b >> shift) & 255) as i32).abs() > 30
            })
        })
        .count();
    let error: usize = cached
        .iter()
        .zip(&direct)
        .map(|(a, b)| {
            [0, 8, 16]
                .iter()
                .map(|shift| {
                    (((*a >> shift) & 255) as i32 - ((*b >> shift) & 255) as i32).unsigned_abs()
                        as usize
                })
                .sum::<usize>()
        })
        .sum();
    let mean_error = error as f64 / (480. * 272. * 3.);
    assert!(
        severe < 480 && mean_error < 2.,
        "Cached settings are corrupted"
    );
    save(
        "PERF-CACHE",
        &serde_json::json!({"different_pixels":differences}),
    );
    drop(cached);
    drop(direct);
    for i in 0..20 {
        crate::settings_scene(renderer, story, &preview, i % 10);
        renderer.present(1);
    }
    let before = renderer.stats();
    let began = crate::now();
    for i in 0..120 {
        crate::settings_scene(renderer, story, &preview, i % 10);
        renderer.present(1);
    }
    let navigation_ms = (crate::now() - began) * 1000. / 120.;
    let after = renderer.stats();
    assert_eq!(before.0, after.0, "Navigating settings reloaded textures");
    assert_eq!(
        before.1, after.1,
        "Navigating settings rebuilt the background"
    );
    screenshot(renderer, "PSP-SETTINGS-NAVIGATION");
    let mut startup = State::default();
    startup.start(story, "start");
    for _ in 0..200 {
        if startup.run(story) == Stop::Say && startup.who == "w" {
            break;
        }
    }
    assert_eq!(startup.who, "w");
    renderer.scene(&startup, story);
    renderer.dialogue(&startup, story, "", usize::MAX);
    let startup_frame = renderer.frame.clone();
    renderer.frame.fade_from(&startup_frame, 0);
    renderer.present(1);
    let startup_before = renderer.stats();
    for alpha in [32, 64, 96, 128, 160, 192, 224, 255] {
        renderer.frame.clone_from(&startup_frame);
        renderer.frame.fade_from(&startup_frame, alpha);
        renderer.present(1);
        if alpha == 128 {
            screenshot(renderer, "PSP-STARTUP-FADE");
        }
    }
    let startup_after = renderer.stats();
    assert_eq!(
        startup_before, startup_after,
        "Startup fade reloaded the scene"
    );
    screenshot(renderer, "PSP-STARTUP-FINAL");
    serde_json::json!({"settings_cache_different_pixels":differences,"settings_cache_mean_rgb_error":mean_error,"settings_cache_severe_pixels":severe,"startup_fade_texture_loads":startup_after.0-startup_before.0,"startup_fade_scene_renders":startup_after.1-startup_before.1,"settings_navigation_ms_per_frame":navigation_ms,"settings_navigation_texture_loads":after.0-before.0,"settings_navigation_scene_renders":after.1-before.1})
}
