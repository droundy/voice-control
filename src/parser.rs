use crate::keys::{char_to_keystrokes, Keystrokes};

type Tokens<'a> = &'a [&'a str];

pub mod number;
pub mod spelling;
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
pub struct Literals {
    tokens: Vec<Vec<&'static str>>,
}
impl FancyParser for Literals {}
impl Parser for Literals {
    type Output = Vec<&'static str>;

    fn parse<'a>(&self, input: Tokens<'a>) -> Option<(Self::Output, Tokens<'a>)> {
        for t in self.tokens.iter() {
            if t.len() <= input.len() {
                let (first, rest) = input.split_at(t.len());
                if t == first {
                    return Some((t.clone(), rest));
                }
            }
        }
        None
    }

    fn possible_starts(&self) -> Vec<&'static str> {
        self.tokens.iter().map(|s| s[0]).collect()
    }
}

pub(crate) fn split_str(s: &'static str) -> Vec<&'static str> {
    let mut toks = Vec::new();
    for w in s.split_whitespace() {
        if w.len() > 0 {
            toks.push(w);
        }
    }
    toks
}

impl Literals {
    fn new(strings: &'static [&'static str]) -> Self {
        let mut tokens = Vec::new();
        for s in strings {
            tokens.push(split_str(s));
        }
        Literals { tokens }
    }
}
impl From<&crate::keys::KeyMapping> for Literals {
    fn from(m: &crate::keys::KeyMapping) -> Self {
        let mut tokens = Vec::new();
        for s in m.all_starts() {
            tokens.push(split_str(s));
        }
        Literals { tokens }
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

// #[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Keys(Vec<Keystrokes>),
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
                crate::keys::send_keystrokes(&s);
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
    let listening = Literals::new(&["start", "stop"]).and_then(
        Literals::new(&["listening"]),
        Box::new(|verb, _| {
            let starting = &verb == &["start"];
            Action::function(move || {
                println!("{} listening", if starting { "start" } else { "stop" });
                AM_LISTENING.store(starting, Ordering::Relaxed)
            })
        }),
    );

    let keymapping = crate::keys::KeyMapping::roundy();
    let letters = Literals::from(&keymapping);
    let spell = Literals::new(&["spell"]).with_repeats(
        letters,
        Box::new(move |_, y: Vec<Vec<&'static str>>| {
            let mut out = Vec::new();
            for c in y {
                out.push(keymapping[&c[..]]);
            }
            Action::Keys(out)
        }),
    );

    let navigation = crate::keys::KeyMapping::navigation();
    let navigation = Literals::from(&navigation).map(Box::new(move |y: Vec<&'static str>| {
        let mut out = Vec::new();
        for c in y {
            out.push(navigation[&c[..]]);
        }
        Action::Keys(out)
    }));

    let dication = Literals::new(&["dictation"]).with_repeats(
        AnyWord,
        Box::new(move |_, y| {
            if y.len() > 0 {
                let mut out = Vec::new();
                for c in y[0].chars() {
                    out.push(char_to_keystrokes(c).expect("bad char 1"));
                }
                y[0].to_string();
                for s in y[1..].iter() {
                    out.push(char_to_keystrokes(' ').expect("bad char 2"));
                    for c in s.chars() {
                        out.push(char_to_keystrokes(c).expect("bad char 3"));
                    }
                }
                Action::Keys(out)
            } else {
                Action::Keys(Vec::new())
            }
        }),
    );

    let mut rules = RuleSet::new();
    rules.add(listening);

    let mut rules_if_listening = RuleSet::new();
    rules_if_listening.add(spell);
    rules_if_listening.add(dication);
    rules_if_listening.add(navigation);
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
    let greeting = Literals::new(&["hello", "hi"]);
    assert_eq!(
        Some((vec!["hello"], &["world"][..])),
        greeting.parse(&["hello", "world"])
    );
    assert_eq!(None, greeting.parse(&["great", "hello", "world"]));
    assert_eq!(
        Some(("hello".to_string(), &["world"][..])),
        greeting
            .map(Box::new(|s: Vec<&'static str>| s[0].to_string()))
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
