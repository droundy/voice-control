type Tokens<'a> = &'a [&'a str];

pub trait Parser: 'static + Send {
    type Output: 'static;
    fn parse<'a>(&self, tokens: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)>;

    fn possible_starts(&self) -> Vec<&'static str>;

    fn change_while_function(
        &mut self,
        _function: &dyn Fn() -> Box<dyn Fn(Tokens) -> bool + Send>,
    ) {
    }
}

pub trait FancyParser: Parser + Sized {
    fn map<U: 'static>(self, function: Box<dyn Fn(Self::Output) -> U + Send>) -> Map<Self, U> {
        Map {
            parser: self,
            function,
        }
    }

    fn unless(self, unless: Box<dyn Fn(Tokens) -> bool + Send + 'static>) -> Unless<Self> {
        Unless {
            parser: self,
            unless,
        }
    }

    fn and_then<T2: Parser, U: 'static>(
        self,
        second: T2,
        function: Box<dyn Fn(Self::Output, T2::Output) -> U + Send>,
    ) -> AndThen<Self, T2, U> {
        AndThen {
            first: self,
            second,
            function,
        }
    }
    fn or<T2: Parser<Output = Self::Output>>(self, second: T2) -> Or<Self, T2> {
        Or {
            first: self,
            second,
        }
    }

    fn many_while(self, function: Box<dyn 'static + Fn(Tokens) -> bool + Send>) -> ManyWhile<Self> {
        ManyWhile {
            parser: self,
            function,
        }
    }

    fn with_repeats<R: Parser, U: 'static>(
        self,
        repeat: R,
        join: Box<dyn 'static + Fn(Self::Output, Vec<R::Output>) -> U + Send>,
    ) -> WithRepeats<Self, R, U> {
        WithRepeats {
            first: self,
            repeat,
            while_function: Box::new(|_| true),
            join,
        }
    }
}

#[derive(Clone)]
pub struct Never;
impl FancyParser for Never {}
impl Parser for Never {
    type Output = ();

    fn parse<'a>(&self, _input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        None
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        Vec::new()
    }
}
pub struct AnyWord;
impl FancyParser for AnyWord {}
impl Parser for AnyWord {
    type Output = String;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        if input.len() > 0 {
            Some((input[0].to_string(), &input[1..]))
        } else {
            None
        }
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

#[derive(Clone)]
pub struct PossibleTokens(&'static [&'static str]);
impl Parser for PossibleTokens {
    type Output = &'static str;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        if let Some((first, rest)) = input.split_first() {
            for t in self.0.iter() {
                if *t == *first {
                    return Some((*t, rest));
                }
            }
        }
        None
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.0.iter().map(|s| *s).collect()
    }
}
impl FancyParser for PossibleTokens {}

#[derive(Clone)]
pub struct Sequence(Vec<PossibleTokens>);

impl FancyParser for Sequence {}
impl Parser for Sequence {
    type Output = Vec<&'static str>;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        let mut rest = input;
        let mut value: Vec<&'static str> = Vec::new();
        for t in self.0.iter() {
            let (next, r) = t.parse(rest)?;
            value.push(next);
            rest = r;
        }
        Some((value, rest))
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.0[0].possible_starts()
    }
}

pub struct AndThen<T1: Parser, T2: Parser, U> {
    first: T1,
    second: T2,
    function: Box<dyn Fn(T1::Output, T2::Output) -> U + Send>,
}

impl<T1: Parser, T2: Parser, U: 'static> FancyParser for AndThen<T1, T2, U> {}
impl<T1: Parser, T2: Parser, U: 'static> Parser for AndThen<T1, T2, U> {
    type Output = U;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        let (v1, rest) = self.first.parse(input)?;
        let (v2, rest) = self.second.parse(rest)?;
        Some(((self.function)(v1, v2), rest))
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.first.possible_starts()
    }
}

pub struct Or<T1, T2> {
    first: T1,
    second: T2,
}

impl<T1: Parser, T2: Parser<Output = T1::Output>> FancyParser for Or<T1, T2> {}
impl<T1: Parser, T2: Parser<Output = T1::Output>> Parser for Or<T1, T2> {
    type Output = T1::Output;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        if let Some(r) = self.first.parse(input) {
            Some(r)
        } else {
            self.second.parse(input)
        }
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.first.possible_starts()
    }
}

pub struct Map<T: Parser, U> {
    parser: T,
    function: Box<dyn Fn(T::Output) -> U + Send>,
}

impl<T: Parser, U: 'static> FancyParser for Map<T, U> {}
impl<T: Parser, U: 'static> Parser for Map<T, U> {
    type Output = U;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        let (intermediate, rest) = self.parser.parse(input)?;
        Some(((self.function)(intermediate), rest))
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.parser.possible_starts()
    }
}

pub struct ManyWhile<T> {
    parser: T,
    function: Box<dyn 'static + Fn(Tokens) -> bool + Send>,
}

impl<T: Parser> FancyParser for ManyWhile<T> {}
impl<T: Parser> Parser for ManyWhile<T> {
    type Output = Vec<T::Output>;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        let mut out = Vec::new();
        let mut rest = input;
        while (self.function)(rest) {
            if let Some((v, r)) = self.parser.parse(rest) {
                out.push(v);
                rest = r;
            } else {
                break;
            }
        }
        Some((out, rest))
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.parser.possible_starts()
    }

    fn change_while_function(&mut self, function: &dyn Fn() -> Box<dyn Fn(Tokens) -> bool + Send>) {
        self.function = function();
    }
}

pub struct WithRepeats<T1: Parser, T2: Parser, U> {
    first: T1,
    repeat: T2,
    while_function: Box<dyn Fn(Tokens) -> bool + Send>,
    join: Box<dyn Fn(T1::Output, Vec<T2::Output>) -> U + Send>,
}
impl<T1: Parser, T2: Parser, U: 'static> FancyParser for WithRepeats<T1, T2, U> {}
impl<T1: Parser, T2: Parser, U: 'static> Parser for WithRepeats<T1, T2, U> {
    type Output = U;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        let (first, mut rest) = self.first.parse(input)?;
        let mut second = Vec::new();
        while (self.while_function)(rest) {
            if let Some((v, r)) = self.repeat.parse(rest) {
                second.push(v);
                rest = r;
            } else {
                break;
            }
        }
        Some(((self.join)(first, second), rest))
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.first.possible_starts()
    }
}

pub struct Unless<T> {
    parser: T,
    unless: Box<dyn Fn(Tokens) -> bool + Send>,
}
impl<T: Parser> Parser for Unless<T> {
    type Output = T::Output;

    fn parse<'a>(&self, tokens: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        if !(self.unless)(tokens) {
            self.parser.parse(tokens)
        } else {
            None
        }
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.parser.possible_starts()
    }

    fn change_while_function(&mut self, function: &dyn Fn() -> Box<dyn Fn(Tokens) -> bool + Send>) {
        self.parser.change_while_function(function)
    }
}

pub struct RuleSet<O> {
    all_rules: Vec<Box<dyn Parser<Output = O>>>,
}
impl<O: 'static> Parser for RuleSet<O> {
    type Output = O;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        for r in self.all_rules.iter() {
            if let Some((v, rest)) = r.parse(input) {
                return Some((v, rest));
            }
        }
        None
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        for r in self.all_rules.iter() {
            for p in r.possible_starts().iter() {
                if !out.contains(p) {
                    out.push(*p);
                }
            }
        }
        out
    }

    fn change_while_function(&mut self, function: &dyn Fn() -> Box<dyn Fn(Tokens) -> bool + Send>) {
        for r in self.all_rules.iter_mut() {
            r.change_while_function(function)
        }
    }
}

impl<O: 'static> RuleSet<O> {
    pub fn new() -> Self {
        RuleSet {
            all_rules: Vec::new(),
        }
    }
    pub fn add<T: Parser<Output = O>>(&mut self, parser: T) {
        self.all_rules.push(Box::new(parser))
    }
    pub fn finish_repeats(&mut self) {
        let start_tokens = self.possible_starts();
        self.change_while_function(&move || {
            let another = start_tokens.clone();
            Box::new(move |t| t.len() > 0 && !another.contains(&&t[0]))
        })
    }
}

pub fn words_to_action(words: &[&'static str]) -> Action {
    let mut out = String::new();
    for w in words.iter() {
        out.push_str(*w);
        out.push(' ')
    }
    Action::Keys(out)
}

// #[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Keys(String),
    // KeyPress,
    // Sequence(Vec<Action>),
    Function(Box<dyn Fn()>),
}
impl Action {
    pub fn function<F: 'static + Fn()>(f: F) -> Self {
        Action::Function(Box::new(f))
    }
    pub fn run(self) {
        match self {
            Action::Keys(s) => {
                println!("typing: {s}");
            }
            Action::Function(f) => {
                f();
            }
        }
    }
}

pub fn my_rules() -> impl Parser<Output = Action> + Send {
    use std::sync::atomic::{AtomicBool, Ordering};
    static AM_LISTENING: AtomicBool = AtomicBool::new(false);
    let listening = PossibleTokens(&["start", "stop"]).and_then(
        PossibleTokens(&["listening"]),
        Box::new(|verb, _| {
            let starting = verb == "start";
            Action::function(move || {
                println!("{} listening", if starting { "start" } else { "stop" });
                AM_LISTENING.store(starting, Ordering::Relaxed)
            })
        }),
    );

    let letters = PossibleTokens(&[
        "alpha", "alfa", "bravo", "brodo", "charlie", "charley", "delta", "echo", "foxtrot",
        "golf", "hotel", "india", "juliett", "kilo", "lima", "mike", "november", "oscar", "papa",
        "quebec", "romeo", "sierra", "tango", "uniform", "victor", "whiskey", "x-ray", "yankee",
        "zulu", "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        "niner",
    ]);
    let spell = PossibleTokens(&["spell"]).with_repeats(
        letters,
        Box::new(|_, y| {
            let mut out = String::new();
            for c in y {
                out.push(match c {
                    "zero" => '0',
                    "one" => '1',
                    "two" => '2',
                    "three" => '3',
                    "four" => '4',
                    "five" => '5',
                    "six" => '6',
                    "seven" => '7',
                    "eight" => '8',
                    "nine" => '9',
                    "niner" => '9',
                    _ => c.chars().next().unwrap(),
                });
            }
            Action::Keys(out)
        }),
    );

    let dication = PossibleTokens(&["dictation"]).with_repeats(
        AnyWord,
        Box::new(|_, y| {
            if y.len() > 0 {
                let mut out = y[0].to_string();
                for c in y[1..].iter() {
                    out.push(' ');
                    out.push_str(c);
                }
                Action::Keys(out)
            } else {
                Action::Keys("".to_string())
            }
        }),
    );

    let mut rules = RuleSet::new();
    rules.add(listening);

    let mut rules_if_listening = RuleSet::new();
    rules_if_listening.add(spell);
    rules_if_listening.add(dication);
    rules_if_listening.finish_repeats();

    rules.add(Unless {
        parser: rules_if_listening,
        unless: Box::new(|_: Tokens| !AM_LISTENING.load(Ordering::Relaxed)),
    });
    rules.finish_repeats();
    rules
}

#[test]
fn test_parser() {
    let greeting = PossibleTokens(&["hello", "hi"]);
    assert_eq!(
        Some(("hello", &["world"][..])),
        greeting.parse(&["hello", "world"])
    );
    assert_eq!(None, greeting.parse(&["great", "hello", "world"]));
    assert_eq!(
        Some(("hello".to_string(), &["world"][..])),
        greeting
            .map(Box::new(|s: &'static str| s.to_string()))
            .parse(&["hello", "world"])
    );

    let _mine = my_rules();
    // assert_eq!(
    //     Some((
    //         Action::Keys("start listening".to_string()),
    //         &["then", "do", "something"][..]
    //     )),
    //     mine.parse(&["start", "listening", "then", "do", "something"])
    // );

    // assert_eq!(
    //     Some((Action::Keys("ac".to_string()), &["and", "more"][..])),
    //     mine.parse(&["spell", "alpha", "charlie", "and", "more"])
    // );
}
