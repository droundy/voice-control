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

pub trait IsParser<T> {
    fn parse<'a>(&mut self, input: &'a str) -> Result<(T, &'a str), Error>;
    fn parse_complete<'a>(&mut self, input: &'a str) -> Result<T, Error> {
        match self.parse(input)? {
            (v, "") => Ok(v),
            _ => Err(Error::Wrong),
        }
    }

    fn describe(&self) -> Description;
}

pub trait IntoParser<T>: Sized + IsParser<T> + 'static {
    fn into_parser(self) -> Parser<T> {
        Parser(P::Raw(Box::new(self)))
    }
}
impl<T, PP: IsParser<T> + 'static> IntoParser<T> for PP {}

pub struct Parser<T>(P<T>);
enum P<T> {
    Raw(Box<dyn IsParser<T>>),
    Option {
        name: String,
        options: Vec<Parser<T>>,
    },
}

struct Map<T, U> {
    parser: Parser<T>,
    f: Box<dyn FnMut(T) -> U>,
}
impl<T, U> IsParser<U> for Map<T, U> {
    fn parse<'a>(&mut self, input: &'a str) -> Result<(U, &'a str), Error> {
        self.parser
            .parse(input)
            .map(|(v, rest)| ((self.f)(v), rest))
    }

    fn describe(&self) -> Description {
        self.parser.describe()
    }
}
impl<T: 'static> Parser<T> {
    pub fn map<U: 'static, F: 'static + FnMut(T) -> U>(self, f: F) -> Parser<U> {
        Map {
            parser: self,
            f: Box::new(f),
        }
        .into_parser()
    }
    pub fn gives<U: 'static + Clone>(self, v: U) -> Parser<U> {
        Map {
            parser: self,
            f: Box::new(move |_| v.clone()),
        }
        .into_parser()
    }
    pub fn join<U: 'static, V: 'static, F: 'static + FnMut(T, U) -> V>(
        self,
        p2: Parser<U>,
        f: F,
    ) -> Parser<V> {
        Join {
            parser1: self,
            parser2: p2,
            join: Box::new(f),
        }
        .into_parser()
    }
}

struct Join<T, U, V> {
    parser1: Parser<T>,
    parser2: Parser<U>,
    join: Box<dyn FnMut(T, U) -> V>,
}
impl<T, U, V> IsParser<V> for Join<T, U, V> {
    fn parse<'a>(&mut self, input: &'a str) -> Result<(V, &'a str), Error> {
        let (v1, input) = self.parser1.parse(input)?;
        let (v2, rest) = self.parser2.parse(input)?;
        Ok(((self.join)(v1, v2), rest))
    }

    fn describe(&self) -> Description {
        let mut d = self.parser1.describe();
        let d2 = self.parser2.describe();
        d.command.push_str(" ");
        d.command.push_str(&d2.command);
        d.patterns.extend(d2.patterns);
        d
    }
}

impl<T> IsParser<T> for Parser<T> {
    fn parse<'a>(&mut self, input: &'a str) -> Result<(T, &'a str), Error> {
        match &mut self.0 {
            P::Raw(p) => p.parse(input),
            P::Option { options, .. } => {
                for parser in options.iter_mut() {
                    match parser.parse(input) {
                        Ok(v) => {
                            return Ok(v);
                        }
                        Err(Error::Incomplete) => {
                            return Err(Error::Incomplete);
                        }
                        Err(Error::Wrong) => (),
                    }
                }
                Err(Error::Wrong)
            }
        }
    }

    fn describe(&self) -> Description {
        match &self.0 {
            P::Raw(p) => p.describe(),
            P::Option { name, options } => {
                let mut commands = Vec::new();
                let mut other_patterns = Vec::new();
                for parser in options.iter() {
                    let d = parser.describe();
                    commands.push(d.command);
                    other_patterns.extend(d.patterns);
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

pub fn choose<T, PP: IntoParser<T>>(name: &str, options: Vec<PP>) -> Parser<T> {
    Parser(P::Option {
        name: name.to_string(),
        options: options.into_iter().map(|p| p.into_parser()).collect(),
    })
}

impl IsParser<()> for &'static str {
    fn parse<'a>(&mut self, input: &'a str) -> Result<((), &'a str), Error> {
        let tag_space = format!("{} ", self);
        if input == *self {
            Ok(((), ""))
        } else if input.starts_with(&tag_space) {
            Ok(((), &input[tag_space.len()..]))
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
