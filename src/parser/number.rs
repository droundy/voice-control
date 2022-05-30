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
        "<digit>",
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

pub fn number() -> Parser<usize> {
    choose("<number>", vec![digit(), teen()])
}

#[test]
fn test() {
    let mut p = number();
    // let e = expect_test::expect![[r#"
    //     <baby actions>

    //     <baby actions>:
    //         nurse
    //         sleep
    //         poop
    //         cry
    // "#]];
    // e.assert_eq(&p.describe().to_string());

    assert_eq!(Ok(1), p.parse_complete("one"));
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
