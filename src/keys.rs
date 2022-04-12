use std::{collections::HashMap, ops::Index};

use rdev::{simulate, EventType, Key, SimulateError};

use crate::parser::split_str;

fn send(event_type: &EventType) {
    let delay = std::time::Duration::from_millis(20);
    match simulate(event_type) {
        Ok(()) => (),
        Err(SimulateError) => {
            println!("We could not send {:?}", event_type);
        }
    }
    // Let ths OS catchup (at least MacOS)
    std::thread::sleep(delay);
}

fn send_key(k: Key) {
    send(&EventType::KeyPress(k));
    send(&EventType::KeyRelease(k));
}
fn send_upper(k: Key) {
    send(&EventType::KeyPress(Key::ShiftLeft));
    send(&EventType::KeyPress(k));
    send(&EventType::KeyRelease(k));
    send(&EventType::KeyRelease(Key::ShiftLeft));
}

/// Send keys to the keyboard
///
/// '🅂' | '⇧' => hold_down(Key::ShiftLeft),
/// '🄰' | '🄾' | '⎇' => hold_down(Key::Alt),
/// '🄲' => hold_down(Key::ControlLeft),
/// '🅆' | '⌘' | '❖' => hold_down(Key::MetaLeft),
/// '🅂' => hold_down(Key::ShiftLeft), //
///
/// '🅃' | '\t' => send_key(Key::Tab),
/// '⎋' | '🄴' => send_key(Key::Escape),
/// '🄱' | '␈' | '⌫' => send_key(Key::Backspace),
/// '⏎' | '\n' | '↵' => send_key(Key::Return),
/// '▤' | '☰' | '𝌆' => send_key(Key::Unknown(135)),
pub fn send_string(s: &str) {
    let mut to_lift = Vec::new();
    {
        let mut hold_down = |k: Key| {
            to_lift.push(k);
            send(&EventType::KeyPress(k));
        };
        for c in s.chars() {
            match char_to_keystrokes(c) {
                Some(Keystrokes::Press(k)) => send_key(k),
                Some(Keystrokes::Shift(k)) => send_upper(k),
                Some(Keystrokes::Down(k)) => hold_down(k),
                None => panic!("invalid key character: {}", c),
            }
        }
        for k in to_lift.iter().rev() {
            send(&EventType::KeyRelease(*k));
        }
    }
}
pub fn send_keystrokes(strokes: &[Keystrokes]) {
    let mut to_lift = Vec::new();
    for k in strokes.iter().copied() {
        match k {
            Keystrokes::Down(k) => {
                to_lift.push(k);
                send(&EventType::KeyPress(k));
            }
            Keystrokes::Press(k) => {
                send(&EventType::KeyPress(k));
                send(&EventType::KeyRelease(k));
            }
            Keystrokes::Shift(k) => {
                send(&EventType::KeyPress(Key::ShiftLeft));
                send(&EventType::KeyPress(k));
                send(&EventType::KeyRelease(k));
                send(&EventType::KeyRelease(Key::ShiftLeft));
            }
        }
    }
    for k in to_lift.iter().rev() {
        send(&EventType::KeyRelease(*k));
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Keystrokes {
    Press(Key),
    Shift(Key),
    Down(Key),
}

pub(crate) fn char_to_keystrokes(c: char) -> Option<Keystrokes> {
    match c {
        '🅂' | '⇧' => Some(Keystrokes::Down(Key::ShiftLeft)),
        '🄰' | '🄾' | '⎇' => Some(Keystrokes::Down(Key::Alt)),
        '🄲' => Some(Keystrokes::Down(Key::ControlLeft)),
        '🅆' | '⌘' | '❖' => Some(Keystrokes::Down(Key::MetaLeft)),

        '🅃' | '\t' => Some(Keystrokes::Press(Key::Tab)),
        '⎋' | '🄴' => Some(Keystrokes::Press(Key::Escape)),
        '🄱' | '␈' | '⌫' => Some(Keystrokes::Press(Key::Backspace)),
        '⏎' | '\n' | '↵' => Some(Keystrokes::Press(Key::Return)),
        '▤' | '☰' | '𝌆' => Some(Keystrokes::Press(Key::Unknown(135))),

        ' ' => Some(Keystrokes::Press(Key::Space)),

        '[' => Some(Keystrokes::Press(Key::LeftBracket)),
        ']' => Some(Keystrokes::Press(Key::RightBracket)),
        '{' => Some(Keystrokes::Shift(Key::LeftBracket)),
        '}' => Some(Keystrokes::Shift(Key::RightBracket)),

        ';' => Some(Keystrokes::Press(Key::SemiColon)),
        ':' => Some(Keystrokes::Shift(Key::SemiColon)),

        '=' => Some(Keystrokes::Press(Key::Equal)),
        '+' => Some(Keystrokes::Shift(Key::Equal)),

        '-' => Some(Keystrokes::Press(Key::Minus)),
        '_' => Some(Keystrokes::Shift(Key::Minus)),

        '\'' => Some(Keystrokes::Press(Key::Quote)),
        '"' => Some(Keystrokes::Shift(Key::Quote)),

        '\\' => Some(Keystrokes::Press(Key::BackSlash)),
        '|' => Some(Keystrokes::Shift(Key::BackSlash)),

        '`' => Some(Keystrokes::Press(Key::BackQuote)),
        '~' => Some(Keystrokes::Shift(Key::BackQuote)),

        '1' => Some(Keystrokes::Press(Key::Num1)),
        '2' => Some(Keystrokes::Press(Key::Num2)),
        '3' => Some(Keystrokes::Press(Key::Num3)),
        '4' => Some(Keystrokes::Press(Key::Num4)),
        '5' => Some(Keystrokes::Press(Key::Num5)),
        '6' => Some(Keystrokes::Press(Key::Num6)),
        '7' => Some(Keystrokes::Press(Key::Num7)),
        '8' => Some(Keystrokes::Press(Key::Num8)),
        '9' => Some(Keystrokes::Press(Key::Num9)),
        '0' => Some(Keystrokes::Press(Key::Num0)),

        '!' => Some(Keystrokes::Shift(Key::Num1)),
        '@' => Some(Keystrokes::Shift(Key::Num2)),
        '#' => Some(Keystrokes::Shift(Key::Num3)),
        '$' => Some(Keystrokes::Shift(Key::Num4)),
        '%' => Some(Keystrokes::Shift(Key::Num5)),
        '^' => Some(Keystrokes::Shift(Key::Num6)),
        '&' => Some(Keystrokes::Shift(Key::Num7)),
        '*' => Some(Keystrokes::Shift(Key::Num8)),
        '(' => Some(Keystrokes::Shift(Key::Num9)),
        ')' => Some(Keystrokes::Shift(Key::Num0)),

        'a' => Some(Keystrokes::Press(Key::KeyA)),
        'b' => Some(Keystrokes::Press(Key::KeyB)),
        'c' => Some(Keystrokes::Press(Key::KeyC)),
        'd' => Some(Keystrokes::Press(Key::KeyD)),
        'e' => Some(Keystrokes::Press(Key::KeyE)),
        'f' => Some(Keystrokes::Press(Key::KeyF)),
        'g' => Some(Keystrokes::Press(Key::KeyG)),
        'h' => Some(Keystrokes::Press(Key::KeyH)),
        'i' => Some(Keystrokes::Press(Key::KeyI)),
        'j' => Some(Keystrokes::Press(Key::KeyJ)),
        'k' => Some(Keystrokes::Press(Key::KeyK)),
        'l' => Some(Keystrokes::Press(Key::KeyL)),
        'm' => Some(Keystrokes::Press(Key::KeyM)),
        'n' => Some(Keystrokes::Press(Key::KeyN)),
        'o' => Some(Keystrokes::Press(Key::KeyO)),
        'p' => Some(Keystrokes::Press(Key::KeyP)),
        'q' => Some(Keystrokes::Press(Key::KeyQ)),
        'r' => Some(Keystrokes::Press(Key::KeyR)),
        's' => Some(Keystrokes::Press(Key::KeyS)),
        't' => Some(Keystrokes::Press(Key::KeyT)),
        'u' => Some(Keystrokes::Press(Key::KeyU)),
        'v' => Some(Keystrokes::Press(Key::KeyV)),
        'w' => Some(Keystrokes::Press(Key::KeyW)),
        'x' => Some(Keystrokes::Press(Key::KeyX)),
        'y' => Some(Keystrokes::Press(Key::KeyY)),
        'z' => Some(Keystrokes::Press(Key::KeyZ)),
        'A' => Some(Keystrokes::Shift(Key::KeyA)),
        'B' => Some(Keystrokes::Shift(Key::KeyB)),
        'C' => Some(Keystrokes::Shift(Key::KeyC)),
        'D' => Some(Keystrokes::Shift(Key::KeyD)),
        'E' => Some(Keystrokes::Shift(Key::KeyE)),
        'F' => Some(Keystrokes::Shift(Key::KeyF)),
        'G' => Some(Keystrokes::Shift(Key::KeyG)),
        'H' => Some(Keystrokes::Shift(Key::KeyH)),
        'I' => Some(Keystrokes::Shift(Key::KeyI)),
        'J' => Some(Keystrokes::Shift(Key::KeyJ)),
        'K' => Some(Keystrokes::Shift(Key::KeyK)),
        'L' => Some(Keystrokes::Shift(Key::KeyL)),
        'M' => Some(Keystrokes::Shift(Key::KeyM)),
        'N' => Some(Keystrokes::Shift(Key::KeyN)),
        'O' => Some(Keystrokes::Shift(Key::KeyO)),
        'P' => Some(Keystrokes::Shift(Key::KeyP)),
        'Q' => Some(Keystrokes::Shift(Key::KeyQ)),
        'R' => Some(Keystrokes::Shift(Key::KeyR)),
        'S' => Some(Keystrokes::Shift(Key::KeyS)),
        'T' => Some(Keystrokes::Shift(Key::KeyT)),
        'U' => Some(Keystrokes::Shift(Key::KeyU)),
        'V' => Some(Keystrokes::Shift(Key::KeyV)),
        'W' => Some(Keystrokes::Shift(Key::KeyW)),
        'X' => Some(Keystrokes::Shift(Key::KeyX)),
        'Y' => Some(Keystrokes::Shift(Key::KeyY)),
        'Z' => Some(Keystrokes::Shift(Key::KeyZ)),
        _ => None,
    }
}

pub struct KeyMapping {
    key_to_str: HashMap<Keystrokes, Vec<Vec<&'static str>>>,
    str_to_key: HashMap<Vec<&'static str>, Keystrokes>,
}

impl KeyMapping {
    pub fn nato() -> Self {
        let mut km = KeyMapping {
            key_to_str: HashMap::new(),
            str_to_key: HashMap::new(),
        };
        km.add("alpha", Keystrokes::Press(Key::KeyA));
        km.add("alfa", Keystrokes::Press(Key::KeyA));
        km.add("bravo", Keystrokes::Press(Key::KeyB));
        km.add("brodo", Keystrokes::Press(Key::KeyB));
        km.add("charlie", Keystrokes::Press(Key::KeyC));
        km.add("charley", Keystrokes::Press(Key::KeyC));
        km.add("delta", Keystrokes::Press(Key::KeyD));
        km.add("echo", Keystrokes::Press(Key::KeyE));
        km.add("foxtrot", Keystrokes::Press(Key::KeyF));
        km.add("fox trot", Keystrokes::Press(Key::KeyF));
        km.add("golf", Keystrokes::Press(Key::KeyG));
        km.add("hotel", Keystrokes::Press(Key::KeyH));
        km.add("india", Keystrokes::Press(Key::KeyI));
        km.add("juliett", Keystrokes::Press(Key::KeyJ));
        km.add("kilo", Keystrokes::Press(Key::KeyK));
        km.add("lima", Keystrokes::Press(Key::KeyL));
        km.add("mike", Keystrokes::Press(Key::KeyM));
        km.add("november", Keystrokes::Press(Key::KeyN));
        km.add("oscar", Keystrokes::Press(Key::KeyO));
        km.add("papa", Keystrokes::Press(Key::KeyP));
        km.add("quebec", Keystrokes::Press(Key::KeyQ));
        km.add("romeo", Keystrokes::Press(Key::KeyR));
        km.add("sierra", Keystrokes::Press(Key::KeyS));
        km.add("tango", Keystrokes::Press(Key::KeyT));
        km.add("uniform", Keystrokes::Press(Key::KeyU));
        km.add("victor", Keystrokes::Press(Key::KeyV));
        km.add("whiskey", Keystrokes::Press(Key::KeyW));
        km.add("x-ray", Keystrokes::Press(Key::KeyX));
        km.add("yankee", Keystrokes::Press(Key::KeyY));
        km.add("zulu", Keystrokes::Press(Key::KeyZ));

        km.add("capital alpha", Keystrokes::Shift(Key::KeyA));
        km.add("capital alfa", Keystrokes::Shift(Key::KeyA));
        km.add("capital bravo", Keystrokes::Shift(Key::KeyB));
        km.add("capital brodo", Keystrokes::Shift(Key::KeyB));
        km.add("capital charlie", Keystrokes::Shift(Key::KeyC));
        km.add("capital charley", Keystrokes::Shift(Key::KeyC));
        km.add("capital delta", Keystrokes::Shift(Key::KeyD));
        km.add("capital echo", Keystrokes::Shift(Key::KeyE));
        km.add("capital foxtrot", Keystrokes::Shift(Key::KeyF));
        km.add("capital fox trot", Keystrokes::Shift(Key::KeyF));
        km.add("capital golf", Keystrokes::Shift(Key::KeyG));
        km.add("capital hotel", Keystrokes::Shift(Key::KeyH));
        km.add("capital india", Keystrokes::Shift(Key::KeyI));
        km.add("capital juliett", Keystrokes::Shift(Key::KeyJ));
        km.add("capital kilo", Keystrokes::Shift(Key::KeyK));
        km.add("capital lima", Keystrokes::Shift(Key::KeyL));
        km.add("capital mike", Keystrokes::Shift(Key::KeyM));
        km.add("capital november", Keystrokes::Shift(Key::KeyN));
        km.add("capital oscar", Keystrokes::Shift(Key::KeyO));
        km.add("capital papa", Keystrokes::Shift(Key::KeyP));
        km.add("capital quebec", Keystrokes::Shift(Key::KeyQ));
        km.add("capital romeo", Keystrokes::Shift(Key::KeyR));
        km.add("capital sierra", Keystrokes::Shift(Key::KeyS));
        km.add("capital tango", Keystrokes::Shift(Key::KeyT));
        km.add("capital uniform", Keystrokes::Shift(Key::KeyU));
        km.add("capital victor", Keystrokes::Shift(Key::KeyV));
        km.add("capital whiskey", Keystrokes::Shift(Key::KeyW));
        km.add("capital x-ray", Keystrokes::Shift(Key::KeyX));
        km.add("capital yankee", Keystrokes::Shift(Key::KeyY));
        km.add("capital zulu", Keystrokes::Shift(Key::KeyZ));

        km.add("zero", Keystrokes::Press(Key::Num0));
        km.add("one", Keystrokes::Press(Key::Num1));
        km.add("two", Keystrokes::Press(Key::Num2));
        km.add("three", Keystrokes::Press(Key::Num3));
        km.add("four", Keystrokes::Press(Key::Num4));
        km.add("five", Keystrokes::Press(Key::Num5));
        km.add("six", Keystrokes::Press(Key::Num6));
        km.add("seven", Keystrokes::Press(Key::Num7));
        km.add("eight", Keystrokes::Press(Key::Num8));
        km.add("nine", Keystrokes::Press(Key::Num9));
        km.add("niner", Keystrokes::Press(Key::Num9));

        km.add("space", Keystrokes::Press(Key::Space));
        km
    }
    pub fn alphabet() -> Self {
        let mut km = KeyMapping {
            key_to_str: HashMap::new(),
            str_to_key: HashMap::new(),
        };
        km.add("a", Keystrokes::Press(Key::KeyA));
        km.add("b", Keystrokes::Press(Key::KeyB));
        km.add("c", Keystrokes::Press(Key::KeyC));
        km.add("d", Keystrokes::Press(Key::KeyD));
        km.add("e", Keystrokes::Press(Key::KeyE));
        km.add("f", Keystrokes::Press(Key::KeyF));
        km.add("g", Keystrokes::Press(Key::KeyG));
        km.add("h", Keystrokes::Press(Key::KeyH));
        km.add("i", Keystrokes::Press(Key::KeyI));
        km.add("j", Keystrokes::Press(Key::KeyJ));
        km.add("k", Keystrokes::Press(Key::KeyK));
        km.add("l", Keystrokes::Press(Key::KeyL));
        km.add("m", Keystrokes::Press(Key::KeyM));
        km.add("n", Keystrokes::Press(Key::KeyN));
        km.add("o", Keystrokes::Press(Key::KeyO));
        km.add("p", Keystrokes::Press(Key::KeyP));
        km.add("q", Keystrokes::Press(Key::KeyQ));
        km.add("r", Keystrokes::Press(Key::KeyR));
        km.add("s", Keystrokes::Press(Key::KeyS));
        km.add("t", Keystrokes::Press(Key::KeyT));
        km.add("u", Keystrokes::Press(Key::KeyU));
        km.add("v", Keystrokes::Press(Key::KeyV));
        km.add("w", Keystrokes::Press(Key::KeyW));
        km.add("x", Keystrokes::Press(Key::KeyX));
        km.add("y", Keystrokes::Press(Key::KeyY));
        km.add("z", Keystrokes::Press(Key::KeyZ));

        km.add("A", Keystrokes::Shift(Key::KeyA));
        km.add("B", Keystrokes::Shift(Key::KeyB));
        km.add("C", Keystrokes::Shift(Key::KeyC));
        km.add("D", Keystrokes::Shift(Key::KeyD));
        km.add("E", Keystrokes::Shift(Key::KeyE));
        km.add("F", Keystrokes::Shift(Key::KeyF));
        km.add("G", Keystrokes::Shift(Key::KeyG));
        km.add("H", Keystrokes::Shift(Key::KeyH));
        km.add("I", Keystrokes::Shift(Key::KeyI));
        km.add("J", Keystrokes::Shift(Key::KeyJ));
        km.add("K", Keystrokes::Shift(Key::KeyK));
        km.add("L", Keystrokes::Shift(Key::KeyL));
        km.add("M", Keystrokes::Shift(Key::KeyM));
        km.add("N", Keystrokes::Shift(Key::KeyN));
        km.add("O", Keystrokes::Shift(Key::KeyO));
        km.add("P", Keystrokes::Shift(Key::KeyP));
        km.add("Q", Keystrokes::Shift(Key::KeyQ));
        km.add("R", Keystrokes::Shift(Key::KeyR));
        km.add("S", Keystrokes::Shift(Key::KeyS));
        km.add("T", Keystrokes::Shift(Key::KeyT));
        km.add("U", Keystrokes::Shift(Key::KeyU));
        km.add("V", Keystrokes::Shift(Key::KeyV));
        km.add("W", Keystrokes::Shift(Key::KeyW));
        km.add("X", Keystrokes::Shift(Key::KeyX));
        km.add("Y", Keystrokes::Shift(Key::KeyY));
        km.add("Z", Keystrokes::Shift(Key::KeyZ));

        km.add("0", Keystrokes::Press(Key::Num0));
        km.add("1", Keystrokes::Press(Key::Num1));
        km.add("2", Keystrokes::Press(Key::Num2));
        km.add("3", Keystrokes::Press(Key::Num3));
        km.add("4", Keystrokes::Press(Key::Num4));
        km.add("5", Keystrokes::Press(Key::Num5));
        km.add("6", Keystrokes::Press(Key::Num6));
        km.add("7", Keystrokes::Press(Key::Num7));
        km.add("8", Keystrokes::Press(Key::Num8));
        km.add("9", Keystrokes::Press(Key::Num9));
        km
    }
    pub fn navigation() -> Self {
        let mut km = KeyMapping {
            key_to_str: HashMap::new(),
            str_to_key: HashMap::new(),
        };

        km.add("shift", Keystrokes::Down(Key::ShiftLeft));
        km.add("shift", Keystrokes::Down(Key::ShiftRight));
        km.add("shift", Keystrokes::Press(Key::ShiftLeft));
        km.add("shift", Keystrokes::Press(Key::ShiftRight));

        km.add("control", Keystrokes::Down(Key::ControlLeft));
        km.add("control", Keystrokes::Down(Key::ControlRight));
        km.add("control", Keystrokes::Press(Key::ControlLeft));
        km.add("control", Keystrokes::Press(Key::ControlRight));

        km.add("alt", Keystrokes::Down(Key::Alt));
        km.add("alt", Keystrokes::Press(Key::Alt));

        km.add("meta", Keystrokes::Down(Key::MetaLeft));
        km.add("meta", Keystrokes::Down(Key::MetaRight));
        km.add("meta", Keystrokes::Press(Key::MetaLeft));
        km.add("meta", Keystrokes::Press(Key::MetaRight));

        km.add("backspace", Keystrokes::Press(Key::Backspace));
        km.add("tab", Keystrokes::Press(Key::Tab));
        km.add("tad", Keystrokes::Press(Key::Tab));
        km.add("menu key", Keystrokes::Press(Key::Unknown(135)));

        km.add("right", Keystrokes::Press(Key::RightArrow));
        km.add("left", Keystrokes::Press(Key::LeftArrow));
        km.add("up", Keystrokes::Press(Key::UpArrow));
        km.add("down", Keystrokes::Press(Key::DownArrow));

        km.add("page up", Keystrokes::Press(Key::PageUp));
        km.add("page down", Keystrokes::Press(Key::PageDown));

        km.add("home key", Keystrokes::Press(Key::Home));
        km.add("end key", Keystrokes::Press(Key::End));
        km
    }
    pub fn roundy() -> Self {
        let mut km = Self::nato();
        km.add("minus", Keystrokes::Press(Key::Minus));
        km.add("underscore", Keystrokes::Shift(Key::Minus));
        km.add("plus", Keystrokes::Shift(Key::Equal));
        km.add("equals", Keystrokes::Press(Key::Equal));
        km.add("equal", Keystrokes::Press(Key::Equal));
        km.add("single quote", Keystrokes::Press(Key::Quote));
        km.add("double quote", Keystrokes::Shift(Key::Quote));
        km.add("back quote", Keystrokes::Press(Key::BackQuote));
        km.add("tilde", Keystrokes::Shift(Key::BackQuote));
        km.add("period", Keystrokes::Press(Key::Dot));
        km.add("comma", Keystrokes::Press(Key::Comma));
        km.add("slash", Keystrokes::Press(Key::Slash));
        km.add("question mark", Keystrokes::Shift(Key::Slash));

        km.add("greater than", Keystrokes::Shift(Key::Dot));
        km.add("wrangle", Keystrokes::Shift(Key::Dot));

        km.add("less than", Keystrokes::Shift(Key::Comma));
        km.add("langle", Keystrokes::Shift(Key::Comma));

        km.add("right parenthesis", Keystrokes::Shift(Key::Num0));
        km.add("right parentheses", Keystrokes::Shift(Key::Num0));
        km.add("right parens", Keystrokes::Shift(Key::Num0));
        km.add("exclamation point", Keystrokes::Shift(Key::Num1));
        km.add("at symbol", Keystrokes::Shift(Key::Num2));
        km.add("pound", Keystrokes::Shift(Key::Num3));
        km.add("dollar sign", Keystrokes::Shift(Key::Num4));
        km.add("percent", Keystrokes::Shift(Key::Num5));
        km.add("caret", Keystrokes::Shift(Key::Num6));
        km.add("carrot", Keystrokes::Shift(Key::Num6));
        km.add("ampersand", Keystrokes::Shift(Key::Num7));
        km.add("asterisk", Keystrokes::Shift(Key::Num8));
        km.add("star", Keystrokes::Shift(Key::Num8));
        km.add("left parenthesis", Keystrokes::Shift(Key::Num9));
        km.add("left parentheses", Keystrokes::Shift(Key::Num9));
        km.add("left parens", Keystrokes::Shift(Key::Num9));

        km.add("shift", Keystrokes::Down(Key::ShiftLeft));
        km.add("shift", Keystrokes::Down(Key::ShiftRight));
        km.add("shift", Keystrokes::Press(Key::ShiftLeft));
        km.add("shift", Keystrokes::Press(Key::ShiftRight));

        km.add("control", Keystrokes::Down(Key::ControlLeft));
        km.add("control", Keystrokes::Down(Key::ControlRight));
        km.add("control", Keystrokes::Press(Key::ControlLeft));
        km.add("control", Keystrokes::Press(Key::ControlRight));

        km.add("alt", Keystrokes::Down(Key::Alt));
        km.add("alt", Keystrokes::Press(Key::Alt));

        km.add("meta", Keystrokes::Down(Key::MetaLeft));
        km.add("meta", Keystrokes::Down(Key::MetaRight));
        km.add("meta", Keystrokes::Press(Key::MetaLeft));
        km.add("meta", Keystrokes::Press(Key::MetaRight));

        km.add("return", Keystrokes::Press(Key::Return));
        km.add("backspace", Keystrokes::Press(Key::Backspace));
        km.add("tab", Keystrokes::Press(Key::Tab));
        km.add("menu key", Keystrokes::Press(Key::Unknown(135)));

        km.add("right", Keystrokes::Press(Key::RightArrow));
        km.add("left", Keystrokes::Press(Key::LeftArrow));
        km.add("up", Keystrokes::Press(Key::UpArrow));
        km.add("down", Keystrokes::Press(Key::DownArrow));

        km.add("page up", Keystrokes::Press(Key::PageUp));
        km.add("page down", Keystrokes::Press(Key::PageDown));

        km.add("home key", Keystrokes::Press(Key::Home));
        km.add("end key", Keystrokes::Press(Key::End));
        km
    }
    pub fn add(&mut self, s: &'static str, k: Keystrokes) {
        let s = split_str(s);
        if !self.str_to_key.contains_key(&s) {
            self.str_to_key.insert(s.clone(), k);
        }
        if let Some(v) = self.key_to_str.get_mut(&k) {
            v.push(s);
        } else {
            self.key_to_str.insert(k, vec![s]);
        }
    }
    pub fn get_str(&self, k: Keystrokes) -> Option<Vec<&'static str>> {
        let strokes = self.key_to_str.get(&k)?;
        Some(strokes[0].clone())
    }
    pub fn all_starts<'a>(&'a self) -> impl 'a + Iterator<Item = &'static str> {
        self.str_to_key.keys().map(|s| s[0])
    }
    pub fn all_str<'a>(&'a self) -> impl 'a + Iterator<Item = Vec<&'static str>> {
        self.str_to_key.keys().map(|s| s.clone())
    }
}
impl Index<Keystrokes> for KeyMapping {
    type Output = Vec<Vec<&'static str>>;

    fn index(&self, index: Keystrokes) -> &Self::Output {
        if let Some(v) = self.key_to_str.get(&index) {
            v
        } else {
            panic!("cannot match key '{:?}'", index)
        }
    }
}
impl Index<&'static str> for KeyMapping {
    type Output = Keystrokes;

    fn index(&self, s: &'static str) -> &Self::Output {
        &self.str_to_key[&split_str(s)]
    }
}
impl Index<&[&'static str]> for KeyMapping {
    type Output = Keystrokes;

    fn index(&self, s: &[&'static str]) -> &Self::Output {
        &self.str_to_key[s]
    }
}

pub fn keystrokes_to_char(k: Keystrokes) -> Option<char> {
    match k {
        Keystrokes::Down(Key::ShiftLeft) => Some('⇧'),
        Keystrokes::Down(Key::Alt) => Some('⎇'),
        Keystrokes::Down(Key::ControlLeft) | Keystrokes::Down(Key::ControlRight) => Some('🄲'),
        Keystrokes::Down(Key::MetaLeft) => Some('❖'),

        Keystrokes::Press(Key::Tab) => Some('\t'),
        Keystrokes::Press(Key::Escape) => Some('🄴'),
        _ => None,
    }
}
