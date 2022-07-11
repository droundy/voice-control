use tinyset::SetUsize;

use crate::parser::Error;

use super::{IntoParser, IsParser};

#[derive(Debug)]
pub enum RegularGrammar {
    Word { bytes: Vec<u8>, position: usize },
    Choice(Vec<RegularGrammar>),
    Many0(Box<RegularGrammar>),
    Phrase(Vec<RegularGrammar>),
}

struct FollowEntry {
    bytenum: usize,
    followed_by: SetUsize,
}

impl RegularGrammar {
    fn is_null(&self) -> bool {
        match self {
            RegularGrammar::Phrase(v) => !v.iter().any(|g| !g.is_null()),
            RegularGrammar::Choice(v) => !v.iter().any(|g| !g.is_null()),
            RegularGrammar::Word { bytes, .. } => bytes.is_empty(),
            _ => false,
        }
    }
    fn simplify(&mut self) {
        match self {
            RegularGrammar::Word { .. } => (),
            RegularGrammar::Choice(v) => {
                for g in v.iter_mut() {
                    g.simplify();
                }
                // Ensure we have at most one null;
                if v.iter().any(|g| g.is_null()) {
                    v.retain(|g| !g.is_null());
                    v.push(RegularGrammar::Phrase(Vec::new()));
                }
            }
            RegularGrammar::Many0(g) => {
                g.simplify();
            }
            RegularGrammar::Phrase(v) if v.len() == 1 => *self = v.pop().unwrap(),
            RegularGrammar::Phrase(v) => {
                for g in v.iter_mut() {
                    g.simplify();
                }
                v.retain(|g| !g.is_null());
            }
        }
    }
    fn nullable(&self) -> bool {
        match self {
            RegularGrammar::Word { bytes, .. } => bytes.is_empty(),
            RegularGrammar::Phrase(v) => v.is_empty(),
            RegularGrammar::Many0(_) => true,
            RegularGrammar::Choice(v) => v.iter().any(|g| g.nullable()),
        }
    }
    fn firstpos(&self) -> SetUsize {
        let out = match self {
            RegularGrammar::Word { position, .. } => [*position].into_iter().collect(),
            RegularGrammar::Phrase(v) => {
                let mut set = SetUsize::new();
                for g in v.iter() {
                    set = set | &g.firstpos();
                    if !g.nullable() {
                        break;
                    }
                }
                set
            }
            RegularGrammar::Many0(g) => g.firstpos(),
            RegularGrammar::Choice(v) => {
                let mut set = SetUsize::new();
                for g in v.iter() {
                    set = set | &g.firstpos();
                }
                set
            }
        };
        // println!("firstpos {self:?}: {out:?}");
        out
    }
    fn lastpos(&self) -> SetUsize {
        let out = match self {
            RegularGrammar::Word { position, bytes } => {
                [*position + bytes.len() - 1].into_iter().collect()
            }
            RegularGrammar::Phrase(v) => {
                let mut set = SetUsize::new();
                for g in v.iter().rev() {
                    set = set | &g.lastpos();
                    if !g.nullable() {
                        break;
                    }
                }
                set
            }
            RegularGrammar::Many0(g) => g.lastpos(),
            RegularGrammar::Choice(v) => {
                let mut set = SetUsize::new();
                for g in v.iter() {
                    set = set | &g.lastpos();
                }
                set
            }
        };
        // println!("lastpos {self:?}: {out:?}");
        out
    }
    fn fill_follow(&self, table: &mut Vec<FollowEntry>) {
        match self {
            RegularGrammar::Word { position, bytes } => {
                while table.len() < *position + bytes.len() {
                    table.push(FollowEntry {
                        bytenum: 0,
                        followed_by: SetUsize::new(),
                    })
                }
                for (i, b) in bytes[..bytes.len() - 1].iter().copied().enumerate() {
                    let num = *position + i;
                    table[num].bytenum = charnum(b);
                    table[num].followed_by = [num + 1].into_iter().collect();
                }
                table[*position + bytes.len() - 1].bytenum = charnum(bytes[bytes.len() - 1]);
            }
            RegularGrammar::Phrase(v) => {
                for g in v.iter() {
                    g.fill_follow(table);
                }
                // println!("after just children");
                // print_follow_table(table);
                let mut lastpos = SetUsize::new();
                for i in 0..v.len() - 1 {
                    let mut beginnings = SetUsize::new();
                    for j in i + 1..v.len() {
                        beginnings = beginnings | &v[j].firstpos();
                        if !v[j].nullable() {
                            break;
                        }
                    }
                    if v[i].nullable() {
                        lastpos = lastpos | &v[i].lastpos();
                    } else {
                        lastpos = v[i].lastpos();
                    }
                    for num in lastpos.iter() {
                        // println!("num in lastpos is {num} with i {i}");
                        table[num].followed_by = table[num].followed_by.clone() | &beginnings;
                    }
                }
                // println!("after both");
                // print_follow_table(table);
            }
            RegularGrammar::Many0(g) => {
                g.fill_follow(table);
                let beginnings = g.firstpos();
                for num in g.lastpos() {
                    table[num].followed_by = table[num].followed_by.clone() | &beginnings;
                }
            }
            RegularGrammar::Choice(v) => {
                for g in v.iter() {
                    g.fill_follow(table);
                }
            }
        }
    }
}

// fn print_follow_table(table: &Vec<FollowEntry>) {
//     for (i, e) in table.iter().enumerate() {
//         println!("{i:2}: {:?} -> {:?}", numchar(e.bytenum), e.followed_by);
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
pub struct State {
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
            states: vec![State::default()],
        }
    }
}

pub struct DFA {
    // TODO: split state so we can have `next` be more memory compact for faster checking.
    states: Vec<State>,
}

impl DFA {
    pub fn check(&self, input: &str) -> Result<(), Error> {
        let mut current_state = 1;
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
}

impl From<RegularGrammar> for DFA {
    fn from(g: RegularGrammar) -> Self {
        let mut follow = Vec::new();
        g.fill_follow(&mut follow);
        // println!("\nfinal follow");
        // print_follow_table(&follow);
        let mut states = Vec::new();
        states.push(State::default());
        let mut sets = Vec::new();
        sets.push(g.firstpos());
        assert_eq!(sets.len(), states.len());
        let mut i = 0;
        while i < sets.len() {
            let positions = sets[i].clone();
            for bytenum in 0..27 {
                // Calculate what could happen after we encounter bytenum.
                let mut finalset = SetUsize::new();
                for p in positions.iter() {
                    if follow[p].bytenum == bytenum {
                        // println!("Found follow for {} at state {i}", numchar(bytenum));
                        finalset = finalset | &follow[p].followed_by;
                    }
                }
                if !finalset.is_empty() {
                    // Locate finalset in our arrays
                    let f = if let Some(f) = sets.iter().enumerate().find_map(|(idx, h)| {
                        if h == &finalset {
                            Some(idx)
                        } else {
                            None
                        }
                    }) {
                        f
                    } else {
                        states.push(State::default());
                        sets.push(finalset);
                        states.len() - 1
                    };
                    states[i].next[bytenum] = f;
                }
            }
            i += 1;
        }
        for i in 0..states.len() {
            states[i].complete = sets[i].contains(0);
            // println!("{i:2} == {:?}: {:?}", sets[i], states[i]);
        }
        DFA { states }
    }
}

impl DFA {
    pub fn encode<P: IntoParser>(parser: P) -> Self {
        let mut next_position = 1;
        let grammar = parser.to_grammar(&mut next_position);
        let mut grammar = RegularGrammar::Phrase(vec![
            grammar,
            RegularGrammar::Word {
                bytes: vec![b'z'],
                position: 0,
            },
        ]);
        // grammar.simplify();
        grammar.into()
    }
}
