use voice_control::desktop_control::Action;
use voice_control::load_voice_control;
use voice_control::parser::{choose, number::digit, number::number, IntoParser, Parser};
use voice_control::parser::{roundy, IsParser};

fn parse_testing() -> Parser<Action> {
    "testing".map(|_| Action::new("Testing!".to_string(), || println!("I am running a test!")))
}

fn parse_testing_mice() -> Parser<Action> {
    choose(
        "command",
        vec![
            "testing"
                .many1()
                .map(|n| Action::new(format!("{n:?}"), || println!("I am running a test!"))),
            number().map(move |n| {
                Action::new("{n} blind mice".to_string(), move || println!("I see {n}"))
            }),
        ],
    )
}

fn parse_digit() -> Parser<Action> {
    digit().map(move |n| Action::new("{n} blind mice".to_string(), move || println!("I see {n}")))
}

fn parse_mice_testing() -> Parser<Action> {
    choose(
        "command",
        vec![
            number().map(move |n| {
                Action::new("{n} blind mice".to_string(), move || println!("I see {n}"))
            }),
            "testing"
                .many1()
                .map(|n| Action::new(format!("{n:?}"), || println!("I am running a test!"))),
        ],
    )
}

fn parse_mice() -> Parser<Action> {
    number().map(move |n| Action::new("{n} blind mice".to_string(), move || println!("I see {n}")))
}

fn bench_recognize(audio: &str, name: &str, parser: impl Fn() -> Parser<Action>) {
    let data = voice_control::load_data(&format!("test-audio/{audio}.wav"));

    let recognizer = load_voice_control(parser);
    println!(
        "   *** {name} *** {}",
        scaling::bench(|| { recognizer(&data) })
    );
}

fn bench_parse(text: &str, name: &str, parser: impl Fn() -> Parser<Action>) {
    let parser = parser();
    println!("   {name}: {}", scaling::bench(|| { parser.parse(text) }));
}

fn main() {
    for text in [
        "t",
        "te",
        "g",
        "testing testing testing",
        "testing",
        "four",
        "bogus",
    ] {
        println!("{text}:");
        bench_parse(text, "testing", parse_testing);
        bench_parse(text, "digit", parse_digit);
        bench_parse(text, "mice", parse_mice);
        bench_parse(text, "testing_mice", parse_testing_mice);
        bench_parse(text, "mice_testing", parse_mice_testing);
        bench_parse(text, "roundy", roundy::parser);
    }
    return;

    for audio in [
        "testing-testing-testing",
        "testing",
        "testing-testing-testing-unrecognized",
    ] {
        println!("{audio}:");
        bench_recognize(audio, "testing", parse_testing);
        bench_recognize(audio, "testing_mice", parse_testing_mice);
        bench_recognize(audio, "mice_testing", parse_mice_testing);
        bench_recognize(audio, "roundy", roundy::parser);
    }
}
