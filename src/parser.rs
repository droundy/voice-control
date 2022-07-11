use std::{collections::HashMap, sync::Arc};

pub mod number;
pub mod roundy;
pub mod spelling;

mod regular;
pub use regular::{State, DFA};

use self::regular::RegularGrammar;

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

    fn could_be_empty(&self) -> bool {
        false
    }

    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar;
}

pub trait IntoParser: Sized + IsParser + 'static {
    fn into_parser(self) -> Parser<Self::Output> {
        Parser {
            inner: P::Raw(Arc::new(self)),
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

    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        self.parser.to_grammar(next_position)
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

    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        let g1 = self.parser1.to_grammar(next_position);
        let g2 = self.parser2.to_grammar(next_position);
        match (g1, g2) {
            (RegularGrammar::Phrase(mut v1), RegularGrammar::Phrase(v2)) => {
                v1.extend(v2);
                RegularGrammar::Phrase(v1)
            }
            (RegularGrammar::Phrase(mut v), g2) => {
                v.push(g2);
                RegularGrammar::Phrase(v)
            }
            (g1, RegularGrammar::Phrase(v2)) => {
                let mut v = Vec::with_capacity(1 + v2.len());
                v.push(g1);
                v.extend(v2);
                RegularGrammar::Phrase(v)
            }
            (g1, g2) => RegularGrammar::Phrase(vec![g1, g2]),
        }
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

    fn could_be_empty(&self) -> bool {
        match &self.inner {
            P::Raw(p) => p.could_be_empty(),
            P::Choose { options, .. } => options.iter().any(|p| p.could_be_empty()),
        }
    }

    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        match &self.inner {
            P::Raw(p) => p.to_grammar(next_position),
            P::Choose { options, .. } => RegularGrammar::Choice(
                options
                    .iter()
                    .map(|p| p.to_grammar(next_position))
                    .collect(),
            ),
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
}

pub fn choose<T, PP: IntoParser<Output = T>>(name: &str, options: Vec<PP>) -> Parser<T> {
    Parser {
        inner: P::Choose {
            name: name.to_string(),
            options: options.into_iter().map(|p| p.into_parser()).collect(),
        },
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
    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        let mut bytes = Vec::with_capacity(self.len() + 1);
        bytes.push(b' ');
        bytes.extend(self.as_bytes());
        let position = *next_position;
        *next_position += bytes.len();
        RegularGrammar::Word { bytes, position }
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
    fn to_grammar(&self, _next_position: &mut usize) -> RegularGrammar {
        RegularGrammar::Phrase(Vec::new())
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

    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        RegularGrammar::Phrase(vec![
            self.0.to_grammar(next_position),
            RegularGrammar::Many0(Box::new(self.0.to_grammar(next_position))),
        ])
    }
}

struct Many0<T>(Parser<T>);

impl<T: 'static> IsParser for Many0<T> {
    type Output = Vec<T>;

    fn could_be_empty(&self) -> bool {
        true
    }

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
    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        RegularGrammar::Many0(Box::new(self.0.to_grammar(next_position)))
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

    fn to_grammar(&self, next_position: &mut usize) -> RegularGrammar {
        RegularGrammar::Choice(vec![
            self.0.to_grammar(next_position),
            RegularGrammar::Phrase(Vec::new()),
        ])
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

#[test]
fn checking() {
    let dfa = DFA::encode("hello");
    println!("Full dfa for hello: {dfa:?}");
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

    println!("\nMoving on to simple repeat");
    let dfa = DFA::encode("fa".then("la".many0()));
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("fa la la la la").is_ok());
    assert!(dfa.check("fa").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("fa la "));
    assert_eq!(Err(Error::Incomplete), dfa.check("fa "));
    assert_eq!(Err(Error::Incomplete), dfa.check("fa la l"));
    assert_eq!(Err(Error::Wrong), dfa.check("fa la fa"));

    let shape_note = choose("<note>", vec!["fa", "so", "la", "mi"]);
    println!("\nMoving on to repeat of a choose");

    let dfa = DFA::encode(shape_note.clone().many0());
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("fa la so la la").is_ok());
    assert!(dfa.check("fa").is_ok());
    assert!(dfa.check("fa fa fa").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("fa la "));
    assert_eq!(Err(Error::Incomplete), dfa.check("fa "));
    assert_eq!(Err(Error::Incomplete), dfa.check("fa la l"));
    assert_eq!(Err(Error::Wrong), dfa.check("fa la do"));

    let dfa = DFA::encode("sing".then(shape_note.clone().many0()));
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("sing fa la so la la").is_ok());
    assert!(dfa.check("sing fa").is_ok());
    assert!(dfa.check("sing fa fa fa").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa la "));
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa "));
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa la l"));
    assert_eq!(Err(Error::Wrong), dfa.check("sing fa la do"));

    let dfa = DFA::encode("sing".then(shape_note.clone().many0().then("done")));
    println!("Full dfa: {dfa:?}");
    assert!(dfa.check("sing fa la so la la done").is_ok());
    assert!(dfa.check("sing fa done").is_ok());
    assert!(dfa.check("sing done").is_ok());
    assert!(dfa.check("sing fa fa fa done").is_ok());
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa la "));
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa "));
    assert_eq!(Ok(()), dfa.check("sing fa la la done"));
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa la la"));
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa la l"));
    assert_eq!(Err(Error::Incomplete), dfa.check("sing fa la do"));
    assert_eq!(Err(Error::Wrong), dfa.check("sing fa la re"));
}
