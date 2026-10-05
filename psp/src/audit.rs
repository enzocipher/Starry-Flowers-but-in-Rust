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
                    if !sample && state.who == "p" {
                        renderer.scene(&state, story);
                        renderer.dialogue(&state, story, "", usize::MAX);
                        screenshot(renderer, &format!("PSP-PASTILLE-{branch}"));
                        assert!(save("AUDIT-SLOT", &state));
                        let restored: State = load("AUDIT-SLOT").expect("Save reload failed");
                        assert_eq!(restored.pc, state.pc);
                        assert_eq!(restored.text, state.text);
                        let mut heart = state.clone();
                        heart.text = format!("{} \u{1f499}", heart.text);
                        renderer.scene(&heart, story);
                        renderer.dialogue(&heart, story, "", usize::MAX);
                        screenshot(renderer, "PSP-HEART");
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
    assert!(save("AUDIT-PREFS", &prefs));
    let restored: Prefs = load("AUDIT-PREFS").unwrap();
    assert_eq!(restored.lang, "es");
    assert_eq!(restored.read, prefs.read);
    crate::menu(
        renderer,
        "Settings",
        &vec![
            "Language: English".into(),
            "Text speed: 40".into(),
            "Auto time: 3".into(),
            "Music volume: 100%".into(),
            "Sound volume: 100%".into(),
            "Skip unseen text: false".into(),
            "Skip after choices: false".into(),
            "Skip transitions: false".into(),
            "Return".into(),
        ],
        0,
    );
    screenshot(renderer, "PSP-SETTINGS");
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
    serde_json::json!({"passed":true,"dialogues":lines,"endings":finishes,"extras":extras,"decisions":decisions,"accessory_screens":outfits,"save_reload":true,"settings_overwrite":true})
}
