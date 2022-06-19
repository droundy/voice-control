//! A parser for David Roundy

use crate::desktop_control::Action;

use crate::newparser::Parser;
use crate::newparser::IntoParser;

pub fn parser() -> Parser<Action> {
    let number = super::number::number();
    number.map(|n| Action::only_log(&format!("{} blind mice!", n)))
}