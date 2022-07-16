use std::ops::Range;

use super::*;

pub fn digit() -> Parser<usize> {
    choose(
        "<digit>",
        vec![
            "zero".into_parser().gives(0),
            "one".into_parser().gives(1),
            "two".into_parser().gives(2),
            "three".into_parser().gives(3),
            "four".into_parser().gives(4),
            "five".into_parser().gives(5),
            "six".into_parser().gives(6),
            "seven".into_parser().gives(7),
            "eight".into_parser().gives(8),
            "nine".into_parser().gives(9),
        ],
    )
}
pub fn counting_digit() -> Parser<usize> {
    choose(
        "<counting digit>",
        vec![
            "one".into_parser().gives(1),
            "two".into_parser().gives(2),
            "three".into_parser().gives(3),
            "four".into_parser().gives(4),
            "five".into_parser().gives(5),
            "six".into_parser().gives(6),
            "seven".into_parser().gives(7),
            "eight".into_parser().gives(8),
            "nine".into_parser().gives(9),
        ],
    )
}
pub fn teen() -> Parser<usize> {
    choose(
        "<teen>",
        vec![
            "ten".into_parser().gives(10),
            "eleven".into_parser().gives(11),
            "twelve".into_parser().gives(12),
            "thirteen".into_parser().gives(13),
            "fourteen".into_parser().gives(14),
            "fifteen".into_parser().gives(15),
            "sixteen".into_parser().gives(16),
            "seventeen".into_parser().gives(17),
            "eighteen".into_parser().gives(18),
            "nineteen".into_parser().gives(19),
        ],
    )
}
pub fn tens() -> Parser<usize> {
    choose(
        "<tens>",
        vec![
            "twenty".into_parser().gives(20),
            "thirty".into_parser().gives(30),
            "fourty".into_parser().gives(40),
            "fifty".into_parser().gives(50),
            "sixty".into_parser().gives(60),
            "seventy".into_parser().gives(70),
            "eighty".into_parser().gives(80),
            "ninety".into_parser().gives(90),
        ],
    )
}

pub fn ten_to_ninetynine() -> Parser<usize> {
    choose(
        "<10-99>",
        vec![
            teen(),
            tens().join(
                choose("<after tens>", vec![counting_digit(), ().gives(0)]),
                |t, d| t + d,
            ),
        ],
    )
}

pub fn one_to_ninetynine() -> Parser<usize> {
    choose("<1-99>", vec![counting_digit(), ten_to_ninetynine()])
}

#[test]
fn test_one_to_ninetynine() {
    let mut p = one_to_ninetynine();

    assert_eq!(Ok(11), p.parse_complete("eleven"));
    assert_eq!(Ok((11, "")), p.parse("eleven"));
    assert_eq!(Ok((20, "")), p.parse("twenty"));
    assert_eq!(Ok((20, "")), p.parse("twenty "));
    assert_eq!(Ok((20, "on")), p.parse("twenty on"));
    assert_eq!(Ok((21, "")), p.parse("twenty one"));
}

pub fn number() -> Parser<usize> {
    number_range(0, 1_000_000)
}

pub fn number_range(mut min: usize, mut max: usize) -> Parser<usize> {
    let mut choices = Vec::new();
    let mut pushval = |v, s: &'static str| {
        if max >= v && min <= v {
            choices.push(s.gives(v));
        }
    };
    if max < 9 {
        pushval(1, "one");
        pushval(2, "two");
        pushval(3, "three");
        pushval(4, "four");
        pushval(5, "five");
        pushval(6, "six");
        pushval(7, "seven");
        pushval(8, "eight");
    } else if max < 19 {
        pushval(11, "eleven");
        pushval(12, "twelve");
        pushval(13, "thirteen");
        pushval(14, "fourteen");
        pushval(15, "fifteen");
        pushval(16, "sixteen");
        pushval(17, "seventeen");
        pushval(18, "eighteen");
        choices.push(counting_digit());
    } else if max < 100 {
        min = std::cmp::min(min, 1);
        max = 99;
        choices.push(one_to_ninetynine());
    } else if max < 1000 {
        min = std::cmp::min(min, 1);
        max = 999;
        choices.push(ten_to_ninetynine());
        choices.push(counting_digit().join(
            choose(
                "<after-counting>",
                vec![
                    "hundred and".then(counting_digit()),
                    "hundred".then(one_to_ninetynine()),
                    "hundred".gives(0),
                    ().gives(1000),
                ],
            ),
            |h, sh| if sh > 100 { h } else { h * 100 + sh },
        ));
    } else {
        min = std::cmp::min(min, 1);
        max = 999_999;
        const NOT: usize = 0xffffff;
        choices.push(number_range(1, 999).join(
            choose(
                "<after-1-999>",
                vec![
                    "thousand and".then(counting_digit()),
                    "thousand".then(number_range(1, 999)),
                    "thousand".gives(0),
                    ().gives(NOT),
                ],
            ),
            |thousand, less| {
                if less > 1000 {
                    thousand
                } else {
                    thousand * 1000 + less
                }
            },
        ));
    };
    if min == 0 {
        choices.push("zero".gives(0));
    }
    choose(&format!("<{min}-{max}>"), choices)
}

impl From<Range<usize>> for Parser<usize> {
    fn from(range: Range<usize>) -> Self {
        number_range(range.start, range.end + 1)
    }
}

#[test]
fn test_number_range() {
    fn confirm(input: &str, value: usize) {
        for min in 0..value {
            for max in value + 1..value + 101 {
                let mut p = number_range(min, max);
                println!("testing {input} -> {value} in range {min}..{max}");
                assert_eq!(Ok(value), p.parse_complete(input));
                assert_eq!(Ok((value, "")), p.parse(input));
                assert_eq!(Ok((value, "")), p.parse(&format!("{input} ")));
                assert_eq!(Ok((value, "x")), p.parse(&format!("{input} x")));
            }
        }
    }
    confirm("one", 1);
    confirm("twenty one", 21);
    confirm("thirty seven", 37);
    confirm("one hundred thirty seven", 137);
    confirm("four hundred and five", 405);
    confirm("three hundred", 300);
    confirm("nine", 9);
}

#[test]
fn test() {
    let mut p = number();

    assert_eq!(Ok(0), p.parse_complete("zero"));
    assert_eq!(Ok(1), p.parse_complete("one"));
    assert_eq!(Ok(21), p.parse_complete("twenty one"));
    assert_eq!(Ok(321), p.parse_complete("three hundred twenty one"));
    assert_eq!(Ok(300), p.parse_complete("three hundred"));
    assert_eq!(
        Ok(3_101),
        p.parse_complete("three thousand one hundred one")
    );
    assert_eq!(
        Ok(300_101),
        p.parse_complete("three hundred thousand one hundred one")
    );
    assert_eq!(
        Ok(300_101),
        p.parse_complete("three hundred thousand one hundred and one")
    );
    assert_eq!(Ok(300_001), p.parse_complete("three hundred thousand one"));
    assert_eq!(Ok(300_000), p.parse_complete("three hundred thousand"));

    assert_eq!(Ok(21), p.parse_complete("twenty one"));
    assert_eq!(Ok((21, "")), p.parse("twenty one"));
    assert_eq!(Ok((20, "")), p.parse("twenty "));

    let e = expect_test::expect![[r#"
        <0-999999>

        <0-999999>: <1-999> <after-1-999> | zero
        <1-999>: <10-99> | <counting digit> <after-counting>
        <10-99>: <teen> | <tens> <after tens>
        <teen>: ten | eleven | twelve | thirteen | fourteen | fifteen | sixteen
            | seventeen | eighteen | nineteen
        <tens>: twenty | thirty | fourty | fifty | sixty | seventy | eighty
            | ninety
        <after tens>: <counting digit> | 
        <counting digit>: one | two | three | four | five | six | seven | eight
            | nine
        <after-counting>: hundred and <counting digit> | hundred <1-99>
            | hundred | 
        <1-99>: <counting digit> | <10-99>
        <after-1-999>: thousand and <counting digit> | thousand <1-999>
            | thousand | 
    "#]];
    e.assert_eq(&p.describe().to_string());
}

#[test]
fn test_teen() {
    let mut p = teen();
    let e = expect_test::expect![[r#"
        <teen>

        <teen>: ten | eleven | twelve | thirteen | fourteen | fifteen | sixteen
            | seventeen | eighteen | nineteen
    "#]];
    e.assert_eq(&p.describe().to_string());

    assert_eq!(Ok(11), p.parse_complete("eleven"));
}
