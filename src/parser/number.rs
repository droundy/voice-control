use crate::newparser::*;

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
        "<ones>",
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

pub fn number() -> Parser<usize> {
    let subhundreds = choose(
        "<subhundreds>",
        vec![
            "".gives(0),
            counting_digit(),
            teen(),
            tens().join(counting_digit(), |a, b| a + b),
        ],
    );
    let hundreds = choose(
        "<hundreds>",
        vec![
            (counting_digit() + "hundred" + subhundreds).map(|((h, _), sh)| h * 100 + sh),
            (counting_digit() + "hundred and" + counting_digit()).map(|((h, _), sh)| h * 100 + sh),
            (counting_digit() + "hundred").map(|(h, _)| h * 100),
            tens().join(counting_digit(), |a, b| a + b),
            digit(),
            teen(),
        ],
    );
    choose(
        "<number>",
        vec![
            (hundreds.clone() + "thousand" + hundreds.clone()).map(|((t, _), h)| t * 1000 + h),
            (hundreds.clone() + "thousand").map(|(t, _)| t * 1000),
            hundreds,
        ],
    )
}

#[test]
fn test() {
    let mut p = number();

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

    let e = expect_test::expect![[r#"
        <number>

        <number>: <hundreds> thousand <hundreds> | <hundreds> thousand
            | <hundreds>
        <hundreds>: <ones> hundred <subhundreds> | <ones> hundred and <ones>
            | <ones> hundred | <tens> <ones> | <digit> | <teen>
        <ones>: one | two | three | four | five | six | seven | eight | nine
        <subhundreds>:  | <ones> | <teen> | <tens> <ones>
        <teen>: ten | eleven | twelve | thirteen | fourteen | fifteen | sixteen
            | seventeen | eighteen | nineteen
        <tens>: twenty | thirty | fourty | fifty | sixty | seventy | eighty
            | ninety
        <digit>: zero | one | two | three | four | five | six | seven | eight
            | nine
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
