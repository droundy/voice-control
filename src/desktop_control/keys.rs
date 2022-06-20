use crate::parser::{IntoParser, Parser};

use super::Action;

use rdev::{simulate, EventType, Key, SimulateError};

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

impl Action {
    /// Send some keystrokes.
    ///
    /// The input `strokes` accepts a range of unicode characters
    /// for special keys.  FIXME document these!
    pub fn keystrokes(strokes: impl IntoIterator<Item = char>) -> Self {
        Action::internal_keystrokes(
            str_to_keystrokes(strokes.into_iter())
                .expect("Action::keystrokes expects valid characters"),
        )
    }

    fn internal_keystrokes(strokes: Vec<Keystrokes>) -> Self {
        Action::new("test".to_string(), move || {
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
        })
    }
}

impl Parser<Vec<char>> {
    pub fn keystrokes(self) -> Parser<Action> {
        self.map(|k| Action::keystrokes(k.into_iter()))
    }
}
impl Parser<Action> {
    pub fn repeated(self) -> Parser<Action> {
        self.many1().map(|actions| Action {
            name: format!("{actions:?}"),
            f: Box::new(move || {
                for v in actions.iter() {
                    v.run();
                }
            }),
        })
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
enum Keystrokes {
    Press(Key),
    Shift(Key),
    Down(Key),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidChar(char),
}

fn str_to_keystrokes(strokes: impl Iterator<Item = char>) -> Result<Vec<Keystrokes>, Error> {
    let mut out = Vec::with_capacity(strokes.size_hint().0 * 2);
    for c in strokes {
        out.push(char_to_keystrokes(c)?);
    }
    Ok(out)
}

fn char_to_keystrokes(c: char) -> Result<Keystrokes, Error> {
    match c {
        '🅂' | '⇧' => Ok(Keystrokes::Down(Key::ShiftLeft)),
        '🄰' | '🄾' | '⎇' => Ok(Keystrokes::Down(Key::Alt)),
        '🄲' => Ok(Keystrokes::Down(Key::ControlLeft)),
        '🅆' | '⌘' | '❖' => Ok(Keystrokes::Down(Key::MetaLeft)),

        '🅃' | '\t' => Ok(Keystrokes::Press(Key::Tab)),
        '⎋' | '🄴' => Ok(Keystrokes::Press(Key::Escape)),
        '🄱' | '␈' | '⌫' => Ok(Keystrokes::Press(Key::Backspace)),
        '⌦' => Ok(Keystrokes::Press(Key::Delete)),
        '⏎' | '\n' | '↵' => Ok(Keystrokes::Press(Key::Return)),
        '▤' | '☰' | '𝌆' => Ok(Keystrokes::Press(Key::Unknown(135))),

        '←' => Ok(Keystrokes::Press(Key::LeftArrow)),
        '→' => Ok(Keystrokes::Press(Key::RightArrow)),
        '↑' => Ok(Keystrokes::Press(Key::UpArrow)),
        '↓' => Ok(Keystrokes::Press(Key::DownArrow)),

        '⇞' | '⬆' => Ok(Keystrokes::Press(Key::PageUp)),
        '⇟' | '⬇' => Ok(Keystrokes::Press(Key::PageDown)),

        '⇱' => Ok(Keystrokes::Press(Key::Home)),
        '⇲' => Ok(Keystrokes::Press(Key::End)),

        ' ' => Ok(Keystrokes::Press(Key::Space)),

        '[' => Ok(Keystrokes::Press(Key::LeftBracket)),
        ']' => Ok(Keystrokes::Press(Key::RightBracket)),
        '{' => Ok(Keystrokes::Shift(Key::LeftBracket)),
        '}' => Ok(Keystrokes::Shift(Key::RightBracket)),

        ';' => Ok(Keystrokes::Press(Key::SemiColon)),
        ':' => Ok(Keystrokes::Shift(Key::SemiColon)),

        '=' => Ok(Keystrokes::Press(Key::Equal)),
        '+' => Ok(Keystrokes::Shift(Key::Equal)),

        '-' => Ok(Keystrokes::Press(Key::Minus)),
        '_' => Ok(Keystrokes::Shift(Key::Minus)),

        '\'' => Ok(Keystrokes::Press(Key::Quote)),
        '"' => Ok(Keystrokes::Shift(Key::Quote)),

        '\\' => Ok(Keystrokes::Press(Key::BackSlash)),
        '|' => Ok(Keystrokes::Shift(Key::BackSlash)),

        '`' => Ok(Keystrokes::Press(Key::BackQuote)),
        '~' => Ok(Keystrokes::Shift(Key::BackQuote)),

        '1' => Ok(Keystrokes::Press(Key::Num1)),
        '2' => Ok(Keystrokes::Press(Key::Num2)),
        '3' => Ok(Keystrokes::Press(Key::Num3)),
        '4' => Ok(Keystrokes::Press(Key::Num4)),
        '5' => Ok(Keystrokes::Press(Key::Num5)),
        '6' => Ok(Keystrokes::Press(Key::Num6)),
        '7' => Ok(Keystrokes::Press(Key::Num7)),
        '8' => Ok(Keystrokes::Press(Key::Num8)),
        '9' => Ok(Keystrokes::Press(Key::Num9)),
        '0' => Ok(Keystrokes::Press(Key::Num0)),

        '!' => Ok(Keystrokes::Shift(Key::Num1)),
        '@' => Ok(Keystrokes::Shift(Key::Num2)),
        '#' => Ok(Keystrokes::Shift(Key::Num3)),
        '$' => Ok(Keystrokes::Shift(Key::Num4)),
        '%' => Ok(Keystrokes::Shift(Key::Num5)),
        '^' => Ok(Keystrokes::Shift(Key::Num6)),
        '&' => Ok(Keystrokes::Shift(Key::Num7)),
        '*' => Ok(Keystrokes::Shift(Key::Num8)),
        '(' => Ok(Keystrokes::Shift(Key::Num9)),
        ')' => Ok(Keystrokes::Shift(Key::Num0)),

        'a' => Ok(Keystrokes::Press(Key::KeyA)),
        'b' => Ok(Keystrokes::Press(Key::KeyB)),
        'c' => Ok(Keystrokes::Press(Key::KeyC)),
        'd' => Ok(Keystrokes::Press(Key::KeyD)),
        'e' => Ok(Keystrokes::Press(Key::KeyE)),
        'f' => Ok(Keystrokes::Press(Key::KeyF)),
        'g' => Ok(Keystrokes::Press(Key::KeyG)),
        'h' => Ok(Keystrokes::Press(Key::KeyH)),
        'i' => Ok(Keystrokes::Press(Key::KeyI)),
        'j' => Ok(Keystrokes::Press(Key::KeyJ)),
        'k' => Ok(Keystrokes::Press(Key::KeyK)),
        'l' => Ok(Keystrokes::Press(Key::KeyL)),
        'm' => Ok(Keystrokes::Press(Key::KeyM)),
        'n' => Ok(Keystrokes::Press(Key::KeyN)),
        'o' => Ok(Keystrokes::Press(Key::KeyO)),
        'p' => Ok(Keystrokes::Press(Key::KeyP)),
        'q' => Ok(Keystrokes::Press(Key::KeyQ)),
        'r' => Ok(Keystrokes::Press(Key::KeyR)),
        's' => Ok(Keystrokes::Press(Key::KeyS)),
        't' => Ok(Keystrokes::Press(Key::KeyT)),
        'u' => Ok(Keystrokes::Press(Key::KeyU)),
        'v' => Ok(Keystrokes::Press(Key::KeyV)),
        'w' => Ok(Keystrokes::Press(Key::KeyW)),
        'x' => Ok(Keystrokes::Press(Key::KeyX)),
        'y' => Ok(Keystrokes::Press(Key::KeyY)),
        'z' => Ok(Keystrokes::Press(Key::KeyZ)),
        'A' => Ok(Keystrokes::Shift(Key::KeyA)),
        'B' => Ok(Keystrokes::Shift(Key::KeyB)),
        'C' => Ok(Keystrokes::Shift(Key::KeyC)),
        'D' => Ok(Keystrokes::Shift(Key::KeyD)),
        'E' => Ok(Keystrokes::Shift(Key::KeyE)),
        'F' => Ok(Keystrokes::Shift(Key::KeyF)),
        'G' => Ok(Keystrokes::Shift(Key::KeyG)),
        'H' => Ok(Keystrokes::Shift(Key::KeyH)),
        'I' => Ok(Keystrokes::Shift(Key::KeyI)),
        'J' => Ok(Keystrokes::Shift(Key::KeyJ)),
        'K' => Ok(Keystrokes::Shift(Key::KeyK)),
        'L' => Ok(Keystrokes::Shift(Key::KeyL)),
        'M' => Ok(Keystrokes::Shift(Key::KeyM)),
        'N' => Ok(Keystrokes::Shift(Key::KeyN)),
        'O' => Ok(Keystrokes::Shift(Key::KeyO)),
        'P' => Ok(Keystrokes::Shift(Key::KeyP)),
        'Q' => Ok(Keystrokes::Shift(Key::KeyQ)),
        'R' => Ok(Keystrokes::Shift(Key::KeyR)),
        'S' => Ok(Keystrokes::Shift(Key::KeyS)),
        'T' => Ok(Keystrokes::Shift(Key::KeyT)),
        'U' => Ok(Keystrokes::Shift(Key::KeyU)),
        'V' => Ok(Keystrokes::Shift(Key::KeyV)),
        'W' => Ok(Keystrokes::Shift(Key::KeyW)),
        'X' => Ok(Keystrokes::Shift(Key::KeyX)),
        'Y' => Ok(Keystrokes::Shift(Key::KeyY)),
        'Z' => Ok(Keystrokes::Shift(Key::KeyZ)),
        _ => Err(Error::InvalidChar(c)),
    }
}
