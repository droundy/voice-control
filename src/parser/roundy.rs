//! A parser for David Roundy

use crate::desktop_control::Action;

use super::number::number;
use super::{choose, spelling, IntoParser, Parser};

pub fn parser() -> Parser<Action> {
    let spell = "spell".then(spelling::extended_nato().many1().keystrokes());
    let key_combo = spelling::modifiers()
        .many1()
        .join(spelling::nato(), |mut v, k| {
            v.push(k);
            Action::keystrokes(v)
        });
    let navigation = spelling::control_keys().many1();
    let navigation = number()
        .optional()
        .join(navigation, |n, strokes| {
            let n = n.unwrap_or(1);
            Action::keystrokes(strokes.repeat(n))
        })
        .repeated();
    choose(
        "command",
        vec![
            spell,
            key_combo,
            navigation,
            (number() + "blind mice").map(|(n, _)| Action::only_log(&format!("{n} blind mice!"))),
            "testing testing testing"
                .map(|_| Action::new("Testing!".to_string(), || println!("I am running a test!"))),
        ],
    )
}
