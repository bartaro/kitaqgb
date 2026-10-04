//! Small deterministic JSON value layer for diagnostics and configuration.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}
impl Value {
    pub fn stringify(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) => n.to_string(),
            Self::String(s) => quote(s),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::stringify).collect::<Vec<_>>().join(",")
            ),
            Self::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{}:{}", quote(k), v.stringify()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut p = Parser {
            s: s.as_bytes(),
            i: 0,
            depth: 0,
        };
        let v = p.value()?;
        p.ws()?;
        if p.i != p.s.len() {
            return Err(format!("unexpected trailing JSON at byte {}", p.i));
        }
        Ok(v)
    }
}
pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
}
impl Parser<'_> {
    fn ws(&mut self) -> Result<(), String> {
        loop {
            while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.s.get(self.i..self.i + 2) == Some(b"//") {
                while self.i < self.s.len() && self.s[self.i] != b'\n' {
                    self.i += 1;
                }
            } else if self.s.get(self.i..self.i + 2) == Some(b"/*") {
                self.i += 2;
                while self.i + 1 < self.s.len() && &self.s[self.i..self.i + 2] != b"*/" {
                    self.i += 1;
                }
                if self.i + 1 >= self.s.len() {
                    return Err("unterminated JSON comment".into());
                }
                self.i += 2;
            } else {
                return Ok(());
            }
        }
    }
    fn value(&mut self) -> Result<Value, String> {
        self.ws()?;
        if self.depth >= 128 {
            return Err("JSON nesting limit exceeded".into());
        }
        self.depth += 1;
        let r = match self.s.get(self.i).copied() {
            Some(b'"') => self.string().map(Value::String),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'-' | b'0'..=b'9') => {
                let begin = self.i;
                self.i += 1;
                while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[begin..self.i])
                    .unwrap()
                    .parse::<i64>()
                    .map(Value::Number)
                    .map_err(|e| e.to_string())
            }
            _ => Err(format!("invalid JSON at byte {}", self.i)),
        };
        self.depth -= 1;
        r
    }
    fn literal(&mut self, s: &[u8], v: Value) -> Result<Value, String> {
        if self.s.get(self.i..self.i + s.len()) == Some(s) {
            self.i += s.len();
            Ok(v)
        } else {
            Err("invalid JSON literal".into())
        }
    }
    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut bytes = Vec::new();
        loop {
            let c = *self.s.get(self.i).ok_or("unterminated JSON string")?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(bytes).map_err(|e| e.to_string()),
                b'\\' => {
                    let e = *self.s.get(self.i).ok_or("unterminated JSON escape")?;
                    self.i += 1;
                    match e {
                        b'"' | b'\\' | b'/' => bytes.push(e),
                        b'n' => bytes.push(b'\n'),
                        b'r' => bytes.push(b'\r'),
                        b't' => bytes.push(b'\t'),
                        b'b' => bytes.push(8),
                        b'f' => bytes.push(12),
                        b'u' => {
                            let a = self.hex4()?;
                            let cp = if (0xd800..=0xdbff).contains(&a) {
                                if self.s.get(self.i..self.i + 2) != Some(b"\\u") {
                                    return Err("missing low surrogate".into());
                                }
                                self.i += 2;
                                let b = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&b) {
                                    return Err("invalid low surrogate".into());
                                }
                                0x10000 + ((a - 0xd800) << 10) + (b - 0xdc00)
                            } else {
                                a
                            };
                            let ch = char::from_u32(cp).ok_or("invalid Unicode scalar")?;
                            let mut buf = [0; 4];
                            bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err("invalid JSON escape".into()),
                    }
                }
                0..=31 => return Err("unescaped control character".into()),
                _ => bytes.push(c),
            }
        }
    }
    fn hex4(&mut self) -> Result<u32, String> {
        let s = self
            .s
            .get(self.i..self.i + 4)
            .ok_or("short Unicode escape")?;
        self.i += 4;
        u32::from_str_radix(std::str::from_utf8(s).map_err(|e| e.to_string())?, 16)
            .map_err(|e| e.to_string())
    }
    fn array(&mut self) -> Result<Value, String> {
        self.i += 1;
        let mut out = Vec::new();
        self.ws()?;
        if self.s.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(Value::Array(out));
        }
        loop {
            out.push(self.value()?);
            self.ws()?;
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Value::Array(out));
                }
                _ => return Err("expected comma or ]".into()),
            }
        }
    }
    fn object(&mut self) -> Result<Value, String> {
        self.i += 1;
        let mut out = BTreeMap::new();
        self.ws()?;
        if self.s.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(Value::Object(out));
        }
        loop {
            self.ws()?;
            if self.s.get(self.i) != Some(&b'"') {
                return Err("expected JSON key".into());
            }
            let k = self.string()?;
            self.ws()?;
            if self.s.get(self.i) != Some(&b':') {
                return Err("expected colon".into());
            }
            self.i += 1;
            let v = self.value()?;
            out.insert(k, v);
            self.ws()?;
            match self.s.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::Object(out));
                }
                _ => return Err("expected comma or }".into()),
            }
        }
    }
}
