//! SavedVariables Lua-subset parser.
//!
//! Parses the WoW client's SavedVariables serialization (`Name = { ... }`)
//! into `serde_json::Value`. Supported subset: nested tables, `"..."` and
//! `'...'` strings with Lua 5.1 escapes, decimal/hex numbers, booleans, nil,
//! `--` line comments and `--[[ ]]` long comments.
//!
//! Safety: recursion depth cap, node budget cap, byte-accurate errors.
//! Anything outside the subset is a hard error, never a silent mis-parse.

use crate::error::{AppError, Result};
use serde_json::{Map, Value};

const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 500_000;

/// Parse `var_name = { ... }` and return the table value.
pub fn parse_saved_variables(input: &str, var_name: &str) -> Result<Value> {
    let mut p = Parser { bytes: input.as_bytes(), pos: 0, nodes: 0 };
    p.skip_trivia();
    let name = p.parse_ident().map_err(|e| p.err(e))?;
    if name != var_name {
        return Err(AppError::bridge(format!(
            "expected SavedVariables table `{var_name}`, found `{name}`"
        )));
    }
    p.skip_trivia();
    p.expect_byte(b'=')?;
    p.skip_trivia();
    let value = p.parse_value(0)?;
    p.skip_trivia();
    if p.pos != p.bytes.len() {
        return Err(p.err("unexpected trailing characters"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    nodes: usize,
}

impl Parser<'_> {
    fn err(&self, msg: impl Into<String>) -> AppError {
        AppError::bridge(format!("parse error at byte {}: {}", self.pos, msg.into()))
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) {
        self.pos += 1;
    }

    fn expect_byte(&mut self, want: u8) -> Result<()> {
        match self.peek() {
            Some(b) if b == want => {
                self.bump();
                Ok(())
            }
            Some(b) => Err(self.err(format!(
                "expected `{}`, found `{}`",
                want as char, b as char
            ))),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            // Whitespace.
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.bump();
            }
            // Comments: `--` line or `--[[ ]]` long.
            if self.bytes.get(self.pos..self.pos + 2) == Some(b"--") {
                if self.bytes.get(self.pos..self.pos + 4) == Some(b"--[[") {
                    self.pos += 4;
                    while self.pos < self.bytes.len()
                        && self.bytes.get(self.pos..self.pos + 2) != Some(b"]]")
                    {
                        self.pos += 1;
                    }
                    self.pos = (self.pos + 2).min(self.bytes.len());
                } else {
                    while !matches!(self.peek(), None | Some(b'\n')) {
                        self.bump();
                    }
                }
                continue;
            }
            break;
        }
    }

    fn parse_ident(&mut self) -> std::result::Result<String, String> {
        let start = self.pos;
        match self.peek() {
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'_') => self.bump(),
            _ => return Err("expected identifier".to_string()),
        }
        while matches!(self.peek(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')) {
            self.bump();
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }

    fn parse_value(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return Err(self.err("table nesting exceeds limit"));
        }
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(self.err("node budget exceeded (file too large?)"));
        }
        // Leading-dot decimals (`.5`) are valid Lua; require a digit after the dot.
        let dot_number = self.peek() == Some(b'.')
            && matches!(self.bytes.get(self.pos + 1), Some(b'0'..=b'9'));
        match self.peek() {
            Some(b'{') => self.parse_table(depth),
            Some(b'"') | Some(b'\'') => Ok(Value::String(self.parse_string()?)),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(b'.') if dot_number => self.parse_number(),
            Some(b't' | b'f' | b'n') => self.parse_keyword(),
            Some(c) => Err(self.err(format!("unexpected character `{c}`"))),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn parse_table(&mut self, depth: usize) -> Result<Value> {
        self.expect_byte(b'{')?;
        let mut array: Vec<Value> = Vec::new();
        let mut object: Map<String, Value> = Map::new();
        let mut has_explicit = false;
        loop {
            self.skip_trivia();
            match self.peek() {
                Some(b'}') => {
                    self.bump();
                    break;
                }
                None => return Err(self.err("unterminated table")),
                _ => {}
            }
            // Try `key = value`: look ahead for `[` literal `]` `=` or ident `=`.
            if let Some(key) = self.try_parse_key()? {
                has_explicit = true;
                self.skip_trivia();
                let value = self.parse_value(depth + 1)?;
                object.insert(key, value);
            } else {
                let value = self.parse_value(depth + 1)?;
                array.push(value);
            }
            self.skip_trivia();
            match self.peek() {
                Some(b',' | b';') => self.bump(),
                Some(b'}') => {}
                None => return Err(self.err("unterminated table")),
                _ => return Err(self.err("expected `,` or `}` in table")),
            }
        }
        if has_explicit {
            // Fold positional entries into the object under 1-based keys so no
            // data is lost; pure arrays stay arrays.
            for (i, v) in array.into_iter().enumerate() {
                object.insert((i + 1).to_string(), v);
            }
            Ok(Value::Object(object))
        } else {
            Ok(Value::Array(array))
        }
    }

    /// If the upcoming tokens form a table key, consume through `=` and return it.
    fn try_parse_key(&mut self) -> Result<Option<String>> {
        let save = self.pos;
        let key = match self.peek() {
            Some(b'[') => {
                self.bump();
                self.skip_trivia();
                let k = match self.peek() {
                    Some(b'"') | Some(b'\'') => self.parse_string()?,
                    Some(b'-' | b'0'..=b'9') => {
                        let n = self.parse_number()?;
                        n.to_string()
                    }
                    _ => {
                        self.pos = save;
                        return Ok(None);
                    }
                };
                self.skip_trivia();
                if self.peek() != Some(b']') {
                    self.pos = save;
                    return Ok(None);
                }
                self.bump();
                k
            }
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'_') => match self.parse_ident() {
                Ok(ident) => ident,
                Err(_) => {
                    self.pos = save;
                    return Ok(None);
                }
            },
            _ => return Ok(None),
        };
        self.skip_trivia();
        if self.peek() == Some(b'=') {
            self.bump();
            self.skip_trivia();
            Ok(Some(key))
        } else {
            // Not a key after all (e.g. a bare identifier value — invalid in our
            // subset, but report it at value position, not here).
            self.pos = save;
            Ok(None)
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        let quote = self.peek().unwrap_or(b'"');
        debug_assert!(quote == b'"' || quote == b'\'');
        self.bump();
        let mut out: Vec<u8> = Vec::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b) if b == quote => {
                    self.bump();
                    break;
                }
                Some(b'\\') => {
                    self.bump();
                    match self.peek() {
                        Some(b'n') => out.push(b'\n'),
                        Some(b't') => out.push(b'\t'),
                        Some(b'r') => out.push(b'\r'),
                        Some(b'a') => out.push(0x07),
                        Some(b'b') => out.push(0x08),
                        Some(b'f') => out.push(0x0C),
                        Some(b'v') => out.push(0x0B),
                        Some(b'\\') => out.push(b'\\'),
                        Some(b'"') => out.push(b'"'),
                        Some(b'\'') => out.push(b'\''),
                        Some(b'\n') => out.push(b'\n'),
                        Some(b'0'..=b'9') => {
                            // Decimal escape \ddd (up to 3 digits).
                            let mut val: u32 = 0;
                            for _ in 0..3 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'9') => {
                                        val = val * 10 + u32::from(d - b'0');
                                        self.bump();
                                    }
                                    _ => break,
                                }
                            }
                            if val > 255 {
                                return Err(self.err("decimal escape out of range"));
                            }
                            out.push(val as u8);
                            continue;
                        }
                        Some(c) => {
                            return Err(self.err(format!("bad string escape `\\{c}`")))
                        }
                        None => return Err(self.err("unterminated string escape")),
                    }
                    self.bump();
                }
                Some(b) => {
                    out.push(b);
                    self.bump();
                }
            }
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn parse_number(&mut self) -> Result<Value> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        // Hex integers.
        if self.bytes.get(self.pos..self.pos + 2) == Some(b"0x")
            || self.bytes.get(self.pos..self.pos + 2) == Some(b"0X")
        {
            self.pos += 2;
            let digits = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')) {
                self.bump();
            }
            if self.pos == digits {
                return Err(self.err("bad hex number"));
            }
            let text = &self.bytes[start..self.pos];
            let neg = text.first() == Some(&b'-');
            let body = if neg { &text[3..] } else { &text[2..] };
            let s = String::from_utf8_lossy(body);
            match i64::from_str_radix(&s, 16) {
                Ok(v) => {
                    let v = if neg { -v } else { v };
                    return Ok(Value::Number(v.into()));
                }
                Err(_) => return Err(self.err("hex number out of range")),
            }
        }
        let mut is_float = false;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        if self.peek() == Some(b'.') {
            is_float = true;
            self.bump();
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.bump();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.bump();
            }
            let digits = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.bump();
            }
            if self.pos == digits {
                return Err(self.err("bad number exponent"));
            }
        }
        if self.pos == start || (self.pos == start + 1 && self.bytes[start] == b'-') {
            return Err(self.err("bad number"));
        }
        // A letter immediately after digits (e.g. `12x`) is a malformed number.
        if matches!(self.peek(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_')) {
            return Err(self.err("bad number suffix"));
        }
        let text = String::from_utf8_lossy(&self.bytes[start..self.pos]);
        if !is_float {
            if let Ok(v) = text.parse::<i64>() {
                return Ok(Value::Number(v.into()));
            }
        }
        match text.parse::<f64>() {
            Ok(v) if v.is_finite() => match serde_json::Number::from_f64(v) {
                Some(n) => Ok(Value::Number(n)),
                None => Err(self.err("number out of range")),
            },
            _ => Err(self.err("bad number")),
        }
    }

    fn parse_keyword(&mut self) -> Result<Value> {
        for (word, value) in
            [("true", Value::Bool(true)), ("false", Value::Bool(false)), ("nil", Value::Null)]
        {
            if self.bytes.get(self.pos..self.pos + word.len()) == Some(word.as_bytes()) {
                let after = self.pos + word.len();
                // `nilx` must not parse as `nil`.
                if let Some(next) = self.bytes.get(after) {
                    if matches!(next, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_') {
                        break;
                    }
                }
                self.pos = after;
                return Ok(value);
            }
        }
        Err(self.err("unexpected identifier (only true/false/nil allowed)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wow_style_tables() {
        let v = parse_saved_variables(
            "GatherOptimizerDB = {\n\t[\"a\"] = 1,\n\t[\"b\"] = \"x\",\n}",
            "GatherOptimizerDB",
        )
        .unwrap();
        assert_eq!(v["a"], Value::from(1));
        assert_eq!(v["b"], Value::from("x"));
    }

    #[test]
    fn parses_arrays_and_nesting() {
        let v = parse_saved_variables("T = { { 1, 2 }, { x = true } }", "T").unwrap();
        assert_eq!(v[0][1], Value::from(2));
        assert_eq!(v[1]["x"], Value::Bool(true));
    }

    #[test]
    fn parses_floats_hex_and_negatives() {
        let v = parse_saved_variables("T = { a = -3, b = 30.2, c = 0xFF, d = 1e3 }", "T")
            .unwrap();
        assert_eq!(v["a"], Value::from(-3));
        assert_eq!(v["b"], Value::from(30.2));
        assert_eq!(v["c"], Value::from(255));
        assert_eq!(v["d"], Value::from(1000.0));
    }

    #[test]
    fn parses_escapes_and_comments() {
        let v = parse_saved_variables(
            "-- lead\nT = { s = \"a\\n\\u0041\" } -- trail",
            "T",
        );
        // \u is NOT a Lua 5.1 escape: must fail loudly, not guess.
        assert!(v.is_err());

        let v = parse_saved_variables("T = { s = \"a\\nb\\\\c\\65\" }", "T").unwrap();
        assert_eq!(v["s"], Value::from("a\nb\\cA"));
    }

    #[test]
    fn rejects_wrong_table_name() {
        assert!(parse_saved_variables("Other = {}", "GatherOptimizerDB").is_err());
    }

    #[test]
    fn rejects_trailing_garbage_and_unterminated() {
        assert!(parse_saved_variables("T = {} extra", "T").is_err());
        assert!(parse_saved_variables("T = { a = 1", "T").is_err());
        assert!(parse_saved_variables("T = { a = f(1) }", "T").is_err());
    }

    #[test]
    fn enforces_depth_limit() {
        let deep = "T = ".to_string() + &"{a=".repeat(100) + "1" + &"}".repeat(100);
        assert!(parse_saved_variables(&deep, "T").is_err());
    }
}
