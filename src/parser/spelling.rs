use crate::newparser::*;

pub fn nato() -> Parser<char> {
    choose(
        "<NATO>",
        vec![
            "alpha".gives('a'),
            "bravo".gives('b'),
            "charlie".gives('c'),
            "delta".gives('d'),
            "echo".gives('e'),
            "foxtrot".gives('f'),
            "golf".gives('g'),
            "hotel".gives('h'),
            "india".gives('i'),
            "juliett".gives('j'),
            "kilo".gives('k'),
            "lima".gives('l'),
            "mike".gives('m'),
            "november".gives('n'),
            "oscar".gives('o'),
            "papa".gives('p'),
            "quebec".gives('q'),
            "romeo".gives('r'),
            "sierra".gives('s'),
            "tango".gives('t'),
            "uniform".gives('u'),
            "victor".gives('v'),
            "whiskey".gives('w'),
            "x-ray".gives('x'),
            "yankee".gives('y'),
            "zulu".gives('z'),
        ],
    )
}

pub fn digit() -> Parser<char> {
    choose(
        "<digit>",
        vec![
            "zero".into_parser().gives('0'),
            "one".into_parser().gives('1'),
            "two".into_parser().gives('2'),
            "three".into_parser().gives('3'),
            "four".into_parser().gives('4'),
            "five".into_parser().gives('5'),
            "six".into_parser().gives('6'),
            "seven".into_parser().gives('7'),
            "eight".into_parser().gives('8'),
            "nine".into_parser().gives('9'),
        ],
    )
}

pub fn extended_nato() -> Parser<char> {
    choose(
        "<char>",
        vec![
            nato(),
            ("big".into_parser() + nato()).map(|(_, c)| c.to_ascii_uppercase()),
            digit(),
        ],
    )
}

#[test]
fn test() {
    let mut p = extended_nato();

    assert_eq!(Ok('b'), p.parse_complete("bravo"));
    assert_eq!(Ok('C'), p.parse_complete("big charlie"));

    let e = expect_test::expect![[r#"
        <char>

        <char>: <NATO> | big <NATO> | <digit>
        <NATO>: alpha | bravo | charlie | delta | echo | foxtrot | golf | hotel
            | india | juliett | kilo | lima | mike | november | oscar | papa
            | quebec | romeo | sierra | tango | uniform | victor | whiskey
            | x-ray | yankee | zulu
        <digit>: zero | one | two | three | four | five | six | seven | eight
            | nine
    "#]];
    e.assert_eq(&p.describe().to_string());
}
