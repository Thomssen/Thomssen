//! Minimal parser for Valve's KeyValues text format (`libraryfolders.vdf`,
//! `appmanifest_*.acf`). Returns a nested map; later duplicate keys win.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Map(BTreeMap<String, Vdf>),
}

impl Vdf {
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Map(m) => m.get(key).or_else(|| m.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn str_at(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }
    pub fn entries(&self) -> Vec<(&String, &Vdf)> {
        match self {
            Vdf::Map(m) => m.iter().collect(),
            _ => vec![],
        }
    }
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

#[derive(Debug, PartialEq)]
enum Tok {
    Str(String),
    Open,
    Close,
}

impl<'a> Lexer<'a> {
    fn next(&mut self) -> Option<Tok> {
        loop {
            while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.i + 1 < self.s.len() && self.s[self.i] == b'/' && self.s[self.i + 1] == b'/' {
                while self.i < self.s.len() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
        match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                Some(Tok::Open)
            }
            b'}' => {
                self.i += 1;
                Some(Tok::Close)
            }
            b'"' => {
                self.i += 1;
                let mut out = Vec::new();
                while self.i < self.s.len() && self.s[self.i] != b'"' {
                    if self.s[self.i] == b'\\' && self.i + 1 < self.s.len() {
                        self.i += 1;
                        out.push(match self.s[self.i] {
                            b'n' => b'\n',
                            b't' => b'\t',
                            c => c, // \\ -> \, \" -> "
                        });
                    } else {
                        out.push(self.s[self.i]);
                    }
                    self.i += 1;
                }
                self.i += 1;
                Some(Tok::Str(String::from_utf8_lossy(&out).into_owned()))
            }
            _ => {
                let st = self.i;
                while self.i < self.s.len() && !self.s[self.i].is_ascii_whitespace() && !matches!(self.s[self.i], b'{' | b'}' | b'"') {
                    self.i += 1;
                }
                Some(Tok::Str(String::from_utf8_lossy(&self.s[st..self.i]).into_owned()))
            }
        }
    }
}

fn parse_map(lx: &mut Lexer, depth: usize) -> Option<BTreeMap<String, Vdf>> {
    if depth > 32 {
        return None;
    }
    let mut m = BTreeMap::new();
    loop {
        match lx.next()? {
            Tok::Close => return Some(m),
            Tok::Open => return None,
            Tok::Str(k) => match lx.next()? {
                Tok::Str(v) => {
                    m.insert(k, Vdf::Str(v));
                }
                Tok::Open => {
                    m.insert(k, Vdf::Map(parse_map(lx, depth + 1)?));
                }
                Tok::Close => return None,
            },
        }
    }
}

pub fn parse(text: &str) -> Option<Vdf> {
    let mut lx = Lexer { s: text.as_bytes(), i: 0 };
    let mut root = BTreeMap::new();
    while let Some(t) = lx.next() {
        let Tok::Str(k) = t else { return None };
        match lx.next()? {
            Tok::Open => {
                root.insert(k, Vdf::Map(parse_map(&mut lx, 0)?));
            }
            Tok::Str(v) => {
                root.insert(k, Vdf::Str(v));
            }
            Tok::Close => return None,
        }
    }
    Some(Vdf::Map(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_libraryfolders() {
        let t = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps" { "730" "30000000000" }
	}
	"1" { "path" "D:\\SteamLibrary" } // comment
}"#;
        let v = parse(t).unwrap();
        let lf = v.get("libraryfolders").unwrap();
        let paths: Vec<&str> = lf.entries().iter().filter_map(|(_, e)| e.str_at("path")).collect();
        assert_eq!(paths, ["C:\\Program Files (x86)\\Steam", "D:\\SteamLibrary"]);
    }

    #[test]
    fn rejects_garbage_without_panicking() {
        assert!(parse("\"a\" {").is_none());
        assert!(parse("}").is_none());
        assert!(parse("").is_some());
        let deep = "\"a\" {".repeat(100);
        assert!(parse(&deep).is_none());
    }
}
