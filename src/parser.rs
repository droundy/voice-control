use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tinyset::SetUsize;

pub mod number;
pub mod roundy;
pub mod spelling;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    Incomplete,
    Wrong,
}

pub struct Packrat {
    failures: Vec<HashMap<String, Error>>,
}
impl Packrat {
    fn check(&self, name: &str, input: &str) -> Result<(), Error> {
        if self.failures.len() > input.len() {
            if let Some(e) = self.failures[input.len()].get(name) {
                return Err(e.clone());
            }
        }
        Ok(())
    }
    fn report(&mut self, name: &str, input: &str, e: Error) {
        if self.failures.len() > input.len() {
            self.failures[input.len()].insert(name.to_string(), e);
        }
    }
}

#[derive(Debug)]
pub struct Description {
    command: String,
    patterns: Vec<(String, Vec<String>)>,
}
impl std::fmt::Display for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;
        writeln!(f, "{}\n", self.command)?;
        for p in self.patterns.iter() {
            let mut line = format!("{}", p.0);
            for c in p.1.iter() {
                if line.len() + c.len() + 3 < 72 && !c.contains(":") {
                    if line.contains(": ") || line.contains("    ") {
                        write!(line, " | {}", c)?;
                    } else {
                        write!(line, ": {}", c)?;
                    }
                } else {
                    f.write_str(&line)?;
                    f.write_str("\n")?;
                    line = format!("    | {}", c);
                }
            }
            f.write_str(&line)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

pub trait IsParser: Sync + Send {
    type Output: 'static;
    fn parse<'a>(&self, input: &'a str) -> Result<(Self::Output, &'a str), Error>;
    fn parse_complete<'a>(&mut self, input: &'a str) -> Result<Self::Output, Error> {
        match self.parse(input)? {
            (v, "") => Ok(v),
            _ => Err(Error::Wrong),
        }
    }

    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error>;

    fn describe(&self) -> Description;

    /// Returns the state for any following parse
    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize;
}

#[derive(Clone)]
pub struct Encoding {
    starting_state: SetUsize,
}

pub trait IntoParser: Sized + IsParser + 'static {
    fn into_parser(self) -> Parser<Self::Output> {
        Parser {
            inner: P::Raw(Arc::new(self)),
            invalid_cache: HashSet::new(),
        }
    }

    fn map<U: 'static, F: 'static + Sync + Send + Fn(Self::Output) -> U>(self, f: F) -> Parser<U> {
        Map {
            parser: self.into_parser(),
            f: Box::new(f),
        }
        .into_parser()
    }
    fn gives<U: 'static + Clone + Sync + Send>(self, v: U) -> Parser<U> {
        Map {
            parser: self.into_parser(),
            f: Box::new(move |_| v.clone()),
        }
        .into_parser()
    }
    fn join<
        P2: IntoParser,
        V: 'static,
        F: 'static + Sync + Send + Fn(Self::Output, P2::Output) -> V,
    >(
        self,
        p2: P2,
        f: F,
    ) -> Parser<V> {
        Join {
            parser1: self.into_parser(),
            parser2: p2.into_parser(),
            join: Box::new(f),
        }
        .into_parser()
    }
    fn then<P2: IntoParser>(self, p2: P2) -> Parser<P2::Output> {
        self.join(p2, |_, v| v)
    }
    fn many1(self) -> Parser<Vec<Self::Output>> {
        Many1(self.into_parser()).into_parser()
    }
    fn many0(self) -> Parser<Vec<Self::Output>> {
        Many0(self.into_parser()).into_parser()
    }
    fn optional(self) -> Parser<Option<Self::Output>> {
        Optional(self.into_parser()).into_parser()
    }
}
impl<PP: IsParser + 'static> IntoParser for PP {}

#[derive(Clone)]
pub struct Parser<T> {
    inner: P<T>,
    invalid_cache: HashSet<String>,
}
#[derive(Clone)]
enum P<T> {
    Raw(Arc<dyn IsParser<Output = T>>),
    Choose {
        name: String,
        options: Vec<Parser<T>>,
    },
}

struct Map<T, U> {
    parser: Parser<T>,
    f: Box<dyn Fn(T) -> U + Sync + Send>,
}
impl<T: 'static, U: 'static> IsParser for Map<T, U> {
    type Output = U;
    fn parse<'a>(&self, input: &'a str) -> Result<(U, &'a str), Error> {
        self.parser
            .parse(input)
            .map(|(v, rest)| ((self.f)(v), rest))
    }

    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        self.parser
            .parse_with_packrat(input, packrat)
            .map(|(v, rest)| ((self.f)(v), rest))
    }

    fn describe(&self) -> Description {
        self.parser.describe()
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        self.parser.encode(dfa, encoding)
    }
}
struct Join<T, U, V> {
    parser1: Parser<T>,
    parser2: Parser<U>,
    join: Box<dyn Fn(T, U) -> V + Sync + Send>,
}
impl<T: 'static, U: 'static, V: 'static> IsParser for Join<T, U, V> {
    type Output = V;
    fn parse<'a>(&self, input: &'a str) -> Result<(V, &'a str), Error> {
        let (v1, input) = self.parser1.parse(input)?;
        let (v2, rest) = self.parser2.parse(input)?;
        Ok(((self.join)(v1, v2), rest))
    }

    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        let (v1, input) = self.parser1.parse_with_packrat(input, packrat)?;
        let (v2, rest) = self.parser2.parse_with_packrat(input, packrat)?;
        Ok(((self.join)(v1, v2), rest))
    }

    fn describe(&self) -> Description {
        let mut d = self.parser1.describe();
        let d2 = self.parser2.describe();
        d.command.push_str(" ");
        d.command.push_str(&d2.command);
        let new_patterns: Vec<_> = d2
            .patterns
            .into_iter()
            .filter(|p| !d.patterns.contains(p))
            .collect();
        d.patterns.extend(new_patterns);
        d
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        let first = Encoding { ..encoding };
        // Require a ' ' before the next pattern.
        let current_states = self.parser1.encode(dfa, first);
        let index = charnum(b' ');
        let mut new_current_state = SetUsize::new();
        let next_available = dfa.states.len();
        for current_state in current_states {
            let next = dfa.states[current_state].next[index];
            if next < dfa.states.len() {
                new_current_state.insert(next);
            } else {
                dfa.states[current_state].next[index] = next_available;
                new_current_state.insert(next_available);
                if dfa.states.len() == next_available {
                    dfa.states.push(State::default());
                }
            }
        }
        // Now encode the second parser
        let second = Encoding {
            starting_state: new_current_state,
            ..encoding
        };
        self.parser2.encode(dfa, second)
    }
}

impl<T: 'static> IsParser for Parser<T> {
    type Output = T;
    fn parse<'a>(&self, input: &'a str) -> Result<(T, &'a str), Error> {
        let mut packrat = Packrat {
            failures: vec![HashMap::new(); input.len()],
        };
        match &self.inner {
            P::Raw(p) => p.parse_with_packrat(input, &mut packrat),
            P::Choose { options, .. } => {
                let mut e = Error::Wrong;
                for parser in options.iter() {
                    match parser.parse_with_packrat(input, &mut packrat) {
                        Ok(v) => {
                            return Ok(v);
                        }
                        Err(Error::Incomplete) => {
                            e = Error::Incomplete;
                        }
                        Err(Error::Wrong) => (),
                    }
                }
                Err(e)
            }
        }
    }

    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        match &self.inner {
            P::Raw(p) => p.parse_with_packrat(input, packrat),
            P::Choose { options, name } => {
                packrat.check(name.as_str(), input)?;
                let mut e = Error::Wrong;
                for parser in options.iter() {
                    match parser.parse_with_packrat(input, packrat) {
                        Ok(v) => {
                            return Ok(v);
                        }
                        Err(Error::Incomplete) => {
                            e = Error::Incomplete;
                        }
                        Err(Error::Wrong) => (),
                    }
                }
                packrat.report(&name, input, e.clone());
                Err(e)
            }
        }
    }

    fn describe(&self) -> Description {
        match &self.inner {
            P::Raw(p) => p.describe(),
            P::Choose { name, options } => {
                let mut commands = Vec::new();
                let mut other_patterns = Vec::new();
                for parser in options.iter() {
                    let d = parser.describe();
                    commands.push(d.command);
                    let new_patterns: Vec<_> = d
                        .patterns
                        .into_iter()
                        .filter(|p| !other_patterns.contains(p))
                        .collect();
                    other_patterns.extend(new_patterns);
                }
                let mut patterns = vec![(name.clone(), commands)];
                patterns.extend(other_patterns);
                Description {
                    command: name.clone(),
                    patterns,
                }
            }
        }
    }

    fn encode(&self, dfa: &mut DFA, mut encoding: Encoding) -> SetUsize {
        match &self.inner {
            P::Raw(p) => p.encode(dfa, encoding),
            P::Choose { options, .. } => {
                assert!(!options.is_empty());
                let mut endings = SetUsize::new();
                for o in options {
                    endings = endings | &o.encode(dfa, encoding.clone());
                }
                endings
            }
        }
    }
}

pub fn choose<T, PP: IntoParser<Output = T>>(name: &str, options: Vec<PP>) -> Parser<T> {
    Parser {
        inner: P::Choose {
            name: name.to_string(),
            options: options.into_iter().map(|p| p.into_parser()).collect(),
        },
        invalid_cache: HashSet::new(),
    }
}

impl IsParser for &'static str {
    type Output = &'static str;
    fn parse<'a>(&self, input: &'a str) -> Result<(&'static str, &'a str), Error> {
        let tag_space = format!("{} ", self);
        if input == *self {
            Ok((*self, ""))
        } else if input.starts_with(&tag_space) {
            Ok((*self, &input[tag_space.len()..]))
        } else if self.starts_with(input) {
            Err(Error::Incomplete)
        } else {
            Err(Error::Wrong)
        }
    }
    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        _packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        match input.len().cmp(&self.len()) {
            std::cmp::Ordering::Equal => {
                if input == *self {
                    Ok((*self, ""))
                } else {
                    Err(Error::Wrong)
                }
            }
            std::cmp::Ordering::Less => {
                if self.starts_with(input) {
                    Err(Error::Incomplete)
                } else {
                    Err(Error::Wrong)
                }
            }
            std::cmp::Ordering::Greater => {
                if input.starts_with(*self) && input.as_bytes()[self.len()] == b' ' {
                    Ok((*self, &input[self.len() + 1..]))
                } else {
                    Err(Error::Wrong)
                }
            }
        }
    }
    fn describe(&self) -> Description {
        Description {
            command: self.to_string(),
            patterns: Vec::new(),
        }
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        assert!(self.len() > 0);
        let mut current_states = encoding.starting_state;
        // First do all but the last byte.
        for b in self.as_bytes()[..self.len() - 1].iter().copied() {
            let index = charnum(b);
            let next_available = dfa.states.len();
            let mut next_states = SetUsize::new();
            for current_state in current_states.iter() {
                let next = dfa.states[current_state].next[index];
                if next < dfa.states.len() {
                    next_states.insert(next);
                } else {
                    dfa.states[current_state].next[index] = next_available;
                    next_states.insert(next_available);
                    if dfa.states.len() == next_available {
                        dfa.states.push(State::default());
                    }
                }
            }
            current_states = next_states;
        }
        // Now encode the last byte...
        let index = charnum(self.as_bytes()[self.len() - 1]);
        let next_available = dfa.states.len();
        let mut next_states = SetUsize::new();
        for current_state in current_states.iter() {
            let next = dfa.states[current_state].next[index];
            if next < dfa.states.len() {
                next_states.insert(next);
            } else {
                dfa.states[current_state].next[index] = next_available;
                next_states.insert(next_available);
                if dfa.states.len() == next_available {
                    dfa.states.push(State::default());
                }
            }
        }
        next_states
    }
}

impl IsParser for () {
    type Output = ();
    fn parse<'a>(&self, input: &'a str) -> Result<(Self::Output, &'a str), Error> {
        Ok(((), input))
    }
    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        _packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        Ok(((), input))
    }
    fn describe(&self) -> Description {
        Description {
            command: "".to_string(),
            patterns: Vec::new(),
        }
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        encoding.starting_state
    }
}

struct Many1<T>(Parser<T>);

impl<T: 'static> IsParser for Many1<T> {
    type Output = Vec<T>;

    fn parse<'a>(&self, input: &'a str) -> Result<(Self::Output, &'a str), Error> {
        let (first, mut input) = self.0.parse(input)?;
        let mut output = vec![first];
        loop {
            match self.0.parse(input) {
                Ok((v, rest)) => {
                    output.push(v);
                    input = rest;
                    if input == "" {
                        return Ok((output, input));
                    }
                }
                Err(Error::Incomplete) => return Err(Error::Incomplete),
                Err(Error::Wrong) => return Ok((output, input)),
            }
        }
    }

    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        let (first, mut input) = self.0.parse_with_packrat(input, packrat)?;
        let mut output = vec![first];
        loop {
            match self.0.parse_with_packrat(input, packrat) {
                Ok((v, rest)) => {
                    output.push(v);
                    input = rest;
                    if input == "" {
                        return Ok((output, input));
                    }
                }
                Err(Error::Incomplete) => return Err(Error::Incomplete),
                Err(Error::Wrong) => return Ok((output, input)),
            }
        }
    }

    fn describe(&self) -> Description {
        let mut d = self.0.describe();
        if d.command.contains(' ') {
            d.command = format!("({})+", d.command);
        } else {
            d.command = format!("{}+", d.command);
        }
        d
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        unimplemented!()
    }
}

struct Many0<T>(Parser<T>);

impl<T: 'static> IsParser for Many0<T> {
    type Output = Vec<T>;

    fn parse<'a>(&self, mut input: &'a str) -> Result<(Self::Output, &'a str), Error> {
        let mut output = Vec::new();
        loop {
            match self.0.parse(input) {
                Ok((v, rest)) => {
                    output.push(v);
                    input = rest;
                    if input == "" {
                        return Ok((output, input));
                    }
                }
                Err(Error::Incomplete) => return Err(Error::Incomplete),
                Err(Error::Wrong) => return Ok((output, input)),
            }
        }
    }

    fn parse_with_packrat<'a>(
        &self,
        mut input: &'a str,
        packrat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        let mut output = Vec::new();
        loop {
            match self.0.parse_with_packrat(input, packrat) {
                Ok((v, rest)) => {
                    output.push(v);
                    input = rest;
                    if input == "" {
                        return Ok((output, input));
                    }
                }
                Err(Error::Incomplete) => return Err(Error::Incomplete),
                Err(Error::Wrong) => return Ok((output, input)),
            }
        }
    }

    fn describe(&self) -> Description {
        let mut d = self.0.describe();
        if d.command.contains(' ') {
            d.command = format!("({})*", d.command);
        } else {
            d.command = format!("{}*", d.command);
        }
        d
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        unimplemented!()
    }
}

struct Optional<T>(Parser<T>);

impl<T: 'static> IsParser for Optional<T> {
    type Output = Option<T>;

    fn parse<'a>(&self, input: &'a str) -> Result<(Self::Output, &'a str), Error> {
        match self.0.parse(input) {
            Ok((v, rest)) => Ok((Some(v), rest)),
            Err(Error::Incomplete) => Err(Error::Incomplete),
            Err(Error::Wrong) => Ok((None, input)),
        }
    }

    fn parse_with_packrat<'a>(
        &self,
        input: &'a str,
        rat: &mut Packrat,
    ) -> Result<(Self::Output, &'a str), Error> {
        match self.0.parse_with_packrat(input, rat) {
            Ok((v, rest)) => Ok((Some(v), rest)),
            Err(Error::Incomplete) => Err(Error::Incomplete),
            Err(Error::Wrong) => Ok((None, input)),
        }
    }

    fn describe(&self) -> Description {
        let mut d = self.0.describe();
        if d.command.contains(' ') {
            d.command = format!("({})?", d.command);
        } else {
            d.command = format!("{}?", d.command);
        }
        d
    }

    fn encode(&self, dfa: &mut DFA, encoding: Encoding) -> SetUsize {
        unimplemented!()
    }
}

impl<T: 'static, P2: IntoParser> std::ops::Add<P2> for Parser<T> {
    type Output = Parser<(T, P2::Output)>;

    fn add(self, rhs: P2) -> Self::Output {
        self.join(rhs, |a, b| (a, b))
    }
}

#[test]
fn test_baby_actions() {
    let mut p = choose("<baby actions>", vec!["nurse", "sleep", "poop", "cry"]);
    let e = expect_test::expect![[r#"
        <baby actions>

        <baby actions>: nurse | sleep | poop | cry
    "#]];

    assert!(p.parse("nurse").is_ok());
    assert!(p.parse("nurse more").is_ok());
    assert!(p.parse_complete("nurse more").is_err());
    assert!(p.parse("poop").is_ok());
    assert_eq!(Err(Error::Incomplete), p.parse("poo"));
    assert_eq!(Err(Error::Wrong), p.parse("pee"));

    // Now with `gives`
    let mut p = choose(
        "<baby actions>",
        vec![
            "nurse".into_parser().gives(1usize),
            "sleep".into_parser().gives(2usize),
            "poop".into_parser().gives(13),
            "cry".into_parser().gives(1usize),
        ],
    );
    e.assert_eq(&p.describe().to_string());

    assert_eq!(Ok(1), p.parse_complete("nurse"));
    assert!(p.parse("nurse more").is_ok());
    assert!(p.parse_complete("nurse more").is_err());
    assert_eq!(Ok(13), p.parse_complete("poop"));
    assert_eq!(Err(Error::Incomplete), p.parse("poo"));
    assert_eq!(Err(Error::Wrong), p.parse("pee"));
}

// pub struct Parser<T> {
//     parse: Box<dyn FnMut(&str) -> Result<(T, &str), Error>>,
// }

// impl<T: 'static> Parser<T> {
//     pub fn and_then<U: 'static>(mut self, mut next: Box<dyn FnMut(T) -> Parser<U>>) -> Parser<U> {
//         Parser {
//             parse: Box::new(move |input| {
//                 let (val, rest) = (self.parse)(input)?;
//                 let mut parser = next(val);
//                 (parser.parse)(rest)
//             }),
//         }
//     }
// }

// pub fn choose<T: 'static>(mut options: Vec<Parser<T>>) -> Parser<T> {
//     Parser {
//         parse: Box::new(move |input| {
//             for parser in options.iter_mut() {
//                 match (parser.parse)(input) {
//                     Ok(v) => {
//                         return Ok(v);
//                     }
//                     Err(Error::Incomplete) => {
//                         return Err(Error::Incomplete);
//                     }
//                     Err(Error::Wrong) => (),
//                 }
//             }
//             Err(Error::Wrong)
//         }),
//     }
// }

// impl From<&'static str> for Parser<&'static str> {
//     fn from(tag: &'static str) -> Self {
//         let tag_space = format!("{} ", tag);
//         Parser {
//             parse: Box::new(move |input| {
//                 if input == tag {
//                     Ok((tag, ""))
//                 } else if input.starts_with(&tag_space) {
//                     Ok((tag, &input[tag_space.len()..]))
//                 } else if tag.starts_with(input) {
//                     Err(Error::Incomplete)
//                 } else {
//                     Err(Error::Wrong)
//                 }
//             }),
//         }
//     }
// }

// impl Parser for &'static str {
//     type Output = &'static str;

//     fn parse<'a>(&self, input: &'a str) -> Result<(Self::Output, &'a str), Error> {
//         if input.starts_with(*self) {
//             Ok((*self, &input[self.len()..]))
//         } else if self.starts_with(input) {
//             Err(Error::Incomplete)
//         } else {
//             Err(Error::Wrong)
//         }
//     }
// }

fn charnum(c: u8) -> usize {
    match c {
        b' ' => 26,
        c if c >= b'a' && c <= b'z' => (c - b'a') as usize,
        _ => panic!("unsupported character {:?}", c as char),
    }
}
fn numchar(n: usize) -> char {
    match n {
        26 => ' ',
        n if n < 26 => (b'a' + n as u8) as char,
        _ => panic!("unsupported character"),
    }
}

struct State {
    /// The pattern could end here with this prefix.
    complete: bool,
    next: [usize; 27],
    breadcrumbs: Vec<(Vec<u8>, Vec<u8>)>,
}
impl Default for State {
    fn default() -> Self {
        State {
            complete: false,
            next: [usize::MAX; 27],
            breadcrumbs: Vec::new(),
        }
    }
}
impl State {
    fn complete() -> Self {
        State {
            complete: true,
            next: [usize::MAX; 27],
            breadcrumbs: Vec::new(),
        }
    }
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.breadcrumbs.is_empty() {
            if self.complete {
                f.write_str("C")?;
            } else {
                f.write_str(" ")?;
            }
            for (which, n) in self.next.iter().copied().enumerate() {
                let c = numchar(which);
                if n < usize::MAX {
                    write!(f, " {c:?} -> {n}")?;
                }
            }
        }
        Ok(())
    }
}
impl std::fmt::Debug for DFA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (n, state) in self.states.iter().enumerate() {
            write!(f, "\n  {n}: {state:?}")?;
        }
        Ok(())
    }
}
impl Default for DFA {
    fn default() -> Self {
        DFA {
            states: vec![State::default(), State::complete()],
        }
    }
}

pub struct DFA {
    // TODO: split state so we can have `next` be more memory compact for faster checking.
    states: Vec<State>,
}

impl DFA {
    pub fn check(&self, input: &str) -> Result<(), Error> {
        let mut current_state = 0;
        for b in input.as_bytes().iter().copied() {
            current_state = self.states[current_state].next[charnum(b)];
            if current_state >= self.states.len() {
                return Err(Error::Wrong);
            }
        }
        if self.states[current_state].complete {
            Ok(())
        } else {
            Err(Error::Incomplete)
        }
    }

    fn encode<P: IsParser>(parser: P) -> Self {
        let mut dfa = DFA::default();
        let encoding = Encoding {
            starting_state: [0].into_iter().collect(),
        };
        for end in parser.encode(&mut dfa, encoding) {
            dfa.states[end].complete = true;
        }
        dfa
    }
}

#[test]
fn checking() {
    let dfa = DFA::encode("hello");
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("hello").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("hell"));
    assert_eq!(Err(Error::Wrong), dfa.check("hello "));
    assert_eq!(Err(Error::Wrong), dfa.check("hello world"));

    println!("\nMoving on to hello world");
    let dfa = DFA::encode("hello".map(|a| a) + "world");
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("hello world").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("hell"));
    assert_eq!(Err(Error::Incomplete), dfa.check("hello "));
    assert_eq!(Err(Error::Wrong), dfa.check("goodbye "));
    assert_eq!(Err(Error::Wrong), dfa.check("hello world i am david"));

    println!("\nMoving on to choose");
    let dfa = DFA::encode(choose("<food>", vec!["broccoli", "kale", "spinach"]));
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("broccoli").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("kal"));
    assert_eq!(Err(Error::Wrong), dfa.check("spinach "));
    assert_eq!(Err(Error::Wrong), dfa.check("goodbye "));
    assert_eq!(Err(Error::Wrong), dfa.check("kale i am david"));

    println!("\nMoving on to choose with substrings");
    let dfa = DFA::encode(choose(
        "<food>",
        vec!["peas", "peas and corn on the cob", "peas and corn"],
    ));
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("peas").is_ok());
    assert!(dfa.check("peas and corn on the cob").is_ok());
    assert!(dfa.check("peas and corn").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("peas and"));
    assert_eq!(Err(Error::Incomplete), dfa.check("peas "));
    assert_eq!(Err(Error::Wrong), dfa.check("peas and corn on the cob "));
    assert_eq!(Err(Error::Wrong), dfa.check("kale i am david"));

    println!("\nMoving on to choose in sequence");
    let dfa = DFA::encode(
        "eat".gives(0) + choose("<food>", vec!["broccoli", "kale", "spinach"]) + "every day",
    );
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("eat broccoli every day").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("eat broccoli"));
    assert_eq!(Err(Error::Incomplete), dfa.check("eat"));
    assert_eq!(Err(Error::Incomplete), dfa.check("eat spi"));
    assert_eq!(Err(Error::Incomplete), dfa.check("eat kale ev"));
    assert_eq!(Err(Error::Wrong), dfa.check("eat candy every day"));

    println!("\nMoving on to choose in parallel");
    let dfa = DFA::encode(
        choose(
            "<healthy activity>",
            vec![
                "eat broccoli and kale and exercize".gives((1, "everything")),
                "eat".gives(0) + choose("<food>", vec!["broccoli", "kale", "spinach"]),
                "exercize".gives((1, "workout")),
            ],
        ) + "every day",
    );
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("eat broccoli every day").is_ok());
    assert!(dfa.check("exercize every day").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("eat broccoli"));
    assert_eq!(Err(Error::Incomplete), dfa.check("eat"));
    assert_eq!(Err(Error::Incomplete), dfa.check("eat spi"));
    assert_eq!(Err(Error::Incomplete), dfa.check("eat kale ev"));
    assert_eq!(Err(Error::Wrong), dfa.check("eat candy every day"));
}
