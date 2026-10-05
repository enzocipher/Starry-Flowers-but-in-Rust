use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Debug)]
pub struct Choice {
    pub text: String,
    pub target: usize,
}
#[derive(Clone, Deserialize, Debug)]
pub struct Op {
    pub op: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub who: String,
    #[serde(default)]
    pub attrs: Vec<String>,
    #[serde(default)]
    pub target: usize,
    #[serde(default)]
    pub condition: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub effect: String,
}
#[derive(Deserialize)]
pub struct Story {
    pub ops: Vec<Op>,
    pub labels: HashMap<String, usize>,
    pub images: HashMap<String, String>,
    pub translations: HashMap<String, HashMap<String, String>>,
    pub accessories: Vec<Vec<String>>,
    pub text_images: HashMap<String, String>,
}
impl Story {
    pub fn load() -> Self {
        serde_json::from_str(include_str!("../story.json")).expect("Invalid compiled story")
    }
    pub fn translate(&self, text: &str, lang: &str) -> String {
        self.translations
            .get(lang)
            .and_then(|m| m.get(text))
            .cloned()
            .unwrap_or_else(|| text.to_owned())
    }
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Sprite {
    pub tag: String,
    pub attrs: Vec<String>,
    pub position: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct State {
    pub pc: usize,
    pub stack: Vec<usize>,
    pub vars: HashMap<String, serde_json::Value>,
    pub sprites: Vec<Sprite>,
    pub who: String,
    pub text: String,
    pub attrs: Vec<String>,
    pub music: String,
    pub sound: String,
    pub acc: [usize; 3],
    pub fave: [usize; 3],
    pub outfit: String,
    pub nvl: Vec<String>,
    pub nvl_mode: bool,
    pub window: bool,
    pub seen: Vec<String>,
    #[serde(default)]
    pub effect: String,
    #[serde(default)]
    pub effect_id: u64,
}
impl Default for State {
    fn default() -> Self {
        Self {
            pc: 0,
            stack: vec![],
            vars: HashMap::new(),
            sprites: vec![],
            who: String::new(),
            text: String::new(),
            attrs: vec![],
            music: String::new(),
            sound: String::new(),
            acc: [0; 3],
            fave: [0; 3],
            outfit: "witch".into(),
            nvl: vec![],
            nvl_mode: false,
            window: true,
            seen: vec![],
            effect: String::new(),
            effect_id: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Stop {
    Say,
    Menu,
    Dress,
    Pause(f64),
    Gallery,
    End,
}
fn truth(v: Option<&serde_json::Value>) -> bool {
    v.map(|v| v.as_bool().unwrap_or_else(|| v.as_i64().unwrap_or(0) != 0))
        .unwrap_or(false)
}
impl State {
    fn set_effect(&mut self, effect: &str) {
        if !effect.is_empty() {
            self.effect = effect.into();
            self.effect_id += 1;
        }
    }
    pub fn start(&mut self, story: &Story, label: &str) {
        self.pc = story.labels[label];
        self.stack.clear();
        self.sprites.clear();
        self.nvl.clear();
    }
    fn condition(&self, c: &str) -> bool {
        if let Some(inner) = c.strip_prefix("not ") {
            return !self.condition(inner);
        }
        if c.starts_with("renpy.variant") {
            return true;
        }
        truth(self.vars.get(c))
    }
    fn assignment(&mut self, s: &str) {
        let Some((key, value)) = s.split_once(" = ") else {
            return;
        };
        if let Some(n) = key
            .strip_prefix("acc")
            .and_then(|n| n.parse::<usize>().ok())
        {
            if (1..=3).contains(&n) {
                self.acc[n - 1] = value.parse().unwrap_or_else(|_| self.fave[n - 1]);
            }
            return;
        }
        if key == "faveacc" {
            self.fave = if value == "[0,0,0]" { [0; 3] } else { self.acc };
            return;
        }
        let val = match value {
            "True" => serde_json::Value::Bool(true),
            "False" => serde_json::Value::Bool(false),
            _ => serde_json::Value::String(value.into()),
        };
        self.vars.insert(key.into(), val);
    }
    pub fn apply_sprite(&mut self, action: &str, arg: &str, story: &Story) {
        if let Some((_, effect)) = arg.split_once(" with ") {
            self.set_effect(effect);
        }
        let parts: Vec<&str> = arg.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        let tag = parts[0];
        if action == "scene" {
            self.sprites.clear();
            self.nvl.clear();
        }
        if action == "hide" {
            self.sprites.retain(|s| s.tag != tag);
            return;
        }
        let end = parts
            .iter()
            .position(|p| ["at", "with", "behind", "onlayer"].contains(p))
            .unwrap_or(parts.len());
        let attributes: Vec<String> = parts[1..end].iter().map(|p| p.to_string()).collect();
        let position = parts
            .iter()
            .position(|p| *p == "at")
            .and_then(|n| parts.get(n + 1))
            .unwrap_or(&"")
            .trim_end_matches(',')
            .to_owned();
        if let Some(sprite) = self.sprites.iter_mut().find(|s| s.tag == tag) {
            if tag == "pastille" {
                merge_attrs(&mut sprite.attrs, &attributes, "pastille", story);
            } else if !attributes.is_empty() {
                sprite.attrs = attributes;
            }
            if !position.is_empty() {
                sprite.position = position;
            }
        } else {
            self.sprites.push(Sprite {
                tag: tag.into(),
                attrs: attributes,
                position,
            });
        }
        if tag == "cg" {
            let name = parts[..end].join(" ");
            if !self.seen.contains(&name) {
                self.seen.push(name)
            }
        }
    }
    pub fn run(&mut self, story: &Story) -> Stop {
        self.effect.clear();
        for _ in 0..10000 {
            let Some(op) = story.ops.get(self.pc) else {
                return Stop::End;
            };
            self.pc += 1;
            match op.op.as_str() {
                "say" => {
                    self.set_effect(&op.effect);
                    if op.who == "extend" {
                        self.text.push_str(&op.text);
                        return Stop::Say;
                    }
                    self.who = op.who.clone();
                    self.text = op.text.clone();
                    self.window = true;
                    if self.who == "w" {
                        merge_attrs(&mut self.attrs, &op.attrs, "side_peri", story);
                    }
                    if self.who == "p" {
                        if let Some(s) = self.sprites.iter_mut().find(|s| s.tag == "pastille") {
                            merge_attrs(&mut s.attrs, &op.attrs, "pastille", story);
                        }
                    }
                    let tag = character_tag(&self.who);
                    if !["w", "p", "n", "", "centered"].contains(&self.who.as_str())
                        && !op.attrs.is_empty()
                    {
                        if let Some(s) = self.sprites.iter_mut().find(|s| s.tag == tag) {
                            s.attrs = op.attrs.clone();
                        }
                    }
                    self.nvl_mode = self.who.is_empty();
                    if self.nvl_mode {
                        self.nvl.push(self.text.clone());
                    }
                    if self.text == "{nw}" {
                        continue;
                    }
                    return Stop::Say;
                }
                "menu" => return Stop::Menu,
                "dress" => {
                    self.outfit = op.text.trim_start_matches("side_peri_outfit_").into();
                    return Stop::Dress;
                }
                "set" => self.assignment(&op.text),
                "if" => {
                    if !self.condition(&op.condition) {
                        self.pc = op.target
                    }
                }
                "goto" => self.pc = op.target,
                "jump" | "call" => {
                    if op.text == "gallery" {
                        return Stop::Gallery;
                    }
                    if op.op == "call" {
                        self.stack.push(self.pc)
                    }
                    self.pc = story.labels[&op.text];
                }
                "return" => {
                    if let Some(pc) = self.stack.pop() {
                        self.pc = pc
                    } else {
                        return Stop::End;
                    }
                }
                "show" | "hide" | "scene" => self.apply_sprite(&op.op, &op.text, story),
                "play" => {
                    let name = op
                        .text
                        .trim_start_matches("audio.")
                        .trim_matches('"')
                        .to_string();
                    if op.channel == "music" {
                        self.music = name
                    } else {
                        self.sound = name
                    }
                }
                "stop" => {
                    if op.channel == "music" {
                        self.music.clear()
                    }
                }
                "nvl" => self.nvl.clear(),
                "window" => self.window = !op.text.starts_with("hide"),
                "with" => self.set_effect(&op.text),
                "pause" => {
                    return Stop::Pause(
                        op.text
                            .split('#')
                            .next()
                            .unwrap_or("")
                            .parse()
                            .unwrap_or(0.0),
                    )
                }
                _ => {}
            }
        }
        panic!("Script loop exceeded instruction limit at {}", self.pc)
    }
}
pub fn character_tag(who: &str) -> &str {
    match who {
        "w" => "peri",
        "p" => "pastille",
        "a" => "astra",
        "c" => "cassia",
        "j" => "jam",
        "k" => "kar",
        "r" => "amaretti",
        "u" => "pumpkin",
        "h" => "him",
        "g" => "gumdrop",
        _ => "",
    }
}
pub fn character_name(who: &str) -> &str {
    match who {
        "w" => "Periwinkle",
        "p" => "Pastille",
        "a" => "Astragalus",
        "c" => "Cassia",
        "j" => "Jam",
        "k" => "Kardaemon",
        "r" => "Amaretti",
        "u" => "Nasty Witch",
        "h" => "Himbo Witch",
        "g" => "Gumdrop",
        _ => "",
    }
}
pub fn merge_attrs(old: &mut Vec<String>, new: &[String], prefix: &str, story: &Story) {
    for group in ["outfit", "face"] {
        for attr in new {
            if story
                .images
                .contains_key(&format!("{prefix}_{group}_{attr}"))
            {
                old.retain(|a| !story.images.contains_key(&format!("{prefix}_{group}_{a}")));
                old.push(attr.clone());
            }
        }
    }
}
pub fn clean_text(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for ch in s.chars() {
        match ch {
            '{' => tag = true,
            '}' => tag = false,
            _ => {
                if !tag {
                    out.push(ch)
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_punches_survive_story_compilation() {
        let story = Story::load();
        let punches: Vec<_> = story
            .ops
            .iter()
            .filter(|op| op.op == "say" && op.effect == "vpunch")
            .collect();
        assert!(
            punches.len() > 70,
            "Only {} shakes preserved",
            punches.len()
        );
        let mut state = State::default();
        state.start(&story, "start");
        for _ in 0..100 {
            let stop = state.run(&story);
            if stop == Stop::Menu {
                state.pc = story.ops[state.pc - 1].choices[1].target;
            }
            if state.effect == "vpunch" {
                assert_eq!(state.who, "p");
                assert_eq!(state.text, "A-AH...!");
                assert!(state.effect_id > 0);
                state.run(&story);
                assert_ne!(state.effect, "vpunch");
                return;
            }
        }
        panic!("First Pastille blush shake was not reached");
    }
    #[test]
    fn complete_story_both_dress_paths() {
        let story = Story::load();
        for choice in 0..2 {
            let mut state = State::default();
            state.start(&story, "start");
            let mut dialogue = 0;
            let mut dress = 0;
            for _ in 0..10000 {
                match state.run(&story) {
                    Stop::Say => dialogue += 1,
                    Stop::Menu => state.pc = story.ops[state.pc - 1].choices[choice].target,
                    Stop::Dress => dress += 1,
                    Stop::End => break,
                    _ => {}
                }
            }
            assert!(dialogue > 1400, "{dialogue}");
            assert!(truth(state.vars.get("persistent.clear")));
            assert_eq!(dress > 0, choice == 0);
        }
    }
    #[test]
    fn extras_return_to_caller() {
        let story = Story::load();
        for label in ["ch0", "ch4b", "ch7b", "ex1", "ex2", "ex3", "ex4", "ex5"] {
            let mut s = State::default();
            s.start(&story, label);
            let mut n = 0;
            for _ in 0..1000 {
                match s.run(&story) {
                    Stop::Say => n += 1,
                    Stop::End => break,
                    _ => {}
                }
            }
            assert!(n > 10, "{label}: {n}");
        }
    }
    #[test]
    fn spanish_and_save_roundtrip() {
        let story = Story::load();
        let mut s = State::default();
        s.start(&story, "start");
        assert_eq!(s.run(&story), Stop::Say);
        assert!(story.translate(&s.text, "es").contains("Hola"));
        let json = serde_json::to_string(&s).unwrap();
        let restored: State = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.pc, s.pc);
        assert_eq!(restored.text, s.text);
    }
}
