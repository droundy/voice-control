use std::sync::Arc;

pub mod number;
pub mod roundy;
pub mod spelling;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Incomplete,
    Wrong,
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

    fn describe(&self) -> Description;
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
    Option {
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

    fn describe(&self) -> Description {
        self.parser.describe()
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
}

impl<T: 'static> IsParser for Parser<T> {
    type Output = T;
    fn parse<'a>(&self, input: &'a str) -> Result<(T, &'a str), Error> {
        match &self.inner {
            P::Raw(p) => p.parse(input),
            P::Option { options, .. } => {
                let mut e = Error::Wrong;
                for parser in options.iter() {
                    match parser.parse(input) {
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

    fn describe(&self) -> Description {
        match &self.inner {
            P::Raw(p) => p.describe(),
            P::Option { name, options } => {
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
        inner: P::Option {
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
    fn describe(&self) -> Description {
        Description {
            command: self.to_string(),
            patterns: Vec::new(),
        }
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

    fn describe(&self) -> Description {
        let mut d = self.0.describe();
        if d.command.contains(' ') {
            d.command = format!("({})+", d.command);
        } else {
            d.command = format!("{}+", d.command);
        }
        d
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

    fn describe(&self) -> Description {
        let mut d = self.0.describe();
        if d.command.contains(' ') {
            d.command = format!("({})*", d.command);
        } else {
            d.command = format!("{}*", d.command);
        }
        d
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

    fn describe(&self) -> Description {
        let mut d = self.0.describe();
        if d.command.contains(' ') {
            d.command = format!("({})?", d.command);
        } else {
            d.command = format!("{}?", d.command);
        }
        d
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
