use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
use std::fmt;

/// Represents a HOCON value with proper type information
#[derive(Debug, Clone, PartialEq)]
pub enum HoconValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Object(IndexMap<String, HoconValue>),
    Array(Vec<HoconValue>),
}

/// Document representing parsed HOCON file
#[derive(Debug, Clone, PartialEq)]
pub struct HoconDocument {
    root: IndexMap<String, HoconValue>,
}

impl HoconDocument {
    pub fn new() -> Self {
        Self {
            root: IndexMap::new(),
        }
    }

    pub fn root(&self) -> &IndexMap<String, HoconValue> {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut IndexMap<String, HoconValue> {
        &mut self.root
    }

    pub fn get(&self, key: &str) -> Option<&HoconValue> {
        self.root.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut HoconValue> {
        self.root.get_mut(key)
    }

    pub fn insert(&mut self, key: String, value: HoconValue) {
        self.root.insert(key, value);
    }

    /// Parse HOCON string into document
    pub fn parse(input: &str) -> Result<Self> {
        let mut parser = HoconParser::new(input);
        parser.parse_document()
    }

    /// Serialize document back to HOCON string
    pub fn to_hocon_string(&self) -> String {
        let mut output = String::new();
        serialize_object(&self.root, &mut output, 0);
        output
    }
}

/// Simple HOCON parser
struct HoconParser {
    input: Vec<char>,
    pos: usize,
}

impl HoconParser {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse_document(&mut self) -> Result<HoconDocument> {
        let mut doc = HoconDocument::new();

        while !self.is_eof() {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            // Parse top-level key-value pair
            let (key, value) = self
                .parse_key_value()
                .context("Failed to parse key-value pair")?;
            doc.insert(key, value);

            self.skip_whitespace();
        }

        Ok(doc)
    }

    fn parse_key_value(&mut self) -> Result<(String, HoconValue)> {
        let key = self.parse_key()?;
        self.skip_whitespace();

        if !self.consume_char(':') {
            bail!("Expected ':' after key '{}'", key);
        }

        self.skip_whitespace();
        let value = self.parse_value()?;

        Ok((key, value))
    }

    fn parse_key(&mut self) -> Result<String> {
        let mut key = String::new();

        while !self.is_eof() {
            let ch = self.peek();
            if ch == ':' || ch.is_whitespace() {
                break;
            }
            key.push(self.next());
        }

        if key.is_empty() {
            bail!("Expected key");
        }

        Ok(key)
    }

    fn parse_value(&mut self) -> Result<HoconValue> {
        self.skip_whitespace();

        if self.is_eof() {
            bail!("Unexpected end of input");
        }

        let ch = self.peek();

        match ch {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => self.parse_quoted_string(),
            '-' | '0'..='9' => self.parse_number(),
            _ => self.parse_unquoted_string(),
        }
    }

    fn parse_object(&mut self) -> Result<HoconValue> {
        if !self.consume_char('{') {
            bail!("Expected '{{'");
        }

        let mut map = IndexMap::new();

        loop {
            self.skip_whitespace();

            if self.peek() == '}' {
                self.next();
                break;
            }

            if self.is_eof() {
                bail!("Unterminated object");
            }

            let (key, value) = self.parse_key_value()?;
            map.insert(key, value);

            self.skip_whitespace();
        }

        Ok(HoconValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<HoconValue> {
        if !self.consume_char('[') {
            bail!("Expected '['");
        }

        let mut array = Vec::new();

        loop {
            self.skip_whitespace();

            if self.peek() == ']' {
                self.next();
                break;
            }

            if self.is_eof() {
                bail!("Unterminated array");
            }

            let value = self.parse_value()?;
            array.push(value);

            self.skip_whitespace();
        }

        Ok(HoconValue::Array(array))
    }

    fn parse_quoted_string(&mut self) -> Result<HoconValue> {
        if !self.consume_char('"') {
            bail!("Expected '\"'");
        }

        let mut s = String::new();

        while !self.is_eof() {
            let ch = self.next();
            if ch == '"' {
                return Ok(HoconValue::String(s));
            }
            if ch == '\\' && !self.is_eof() {
                let escaped = self.next();
                match escaped {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    _ => {
                        s.push('\\');
                        s.push(escaped);
                    }
                }
            } else {
                s.push(ch);
            }
        }

        bail!("Unterminated string");
    }

    fn parse_unquoted_string(&mut self) -> Result<HoconValue> {
        let mut s = String::new();

        while !self.is_eof() {
            let ch = self.peek();
            if ch == '\n' || ch == '\r' || ch == '}' || ch == ']' {
                break;
            }
            s.push(self.next());
        }

        let s = s.trim().to_string();

        if s.is_empty() {
            bail!("Expected value");
        }

        Ok(HoconValue::String(s))
    }

    fn parse_number(&mut self) -> Result<HoconValue> {
        let mut num_str = String::new();
        let mut is_float = false;

        if self.peek() == '-' {
            num_str.push(self.next());
        }

        while !self.is_eof() {
            let ch = self.peek();
            if ch.is_ascii_digit() {
                num_str.push(self.next());
            } else if ch == '.' && !is_float {
                is_float = true;
                num_str.push(self.next());
            } else {
                break;
            }
        }

        if is_float {
            num_str
                .parse::<f64>()
                .map(HoconValue::Float)
                .context("Failed to parse float")
        } else {
            num_str
                .parse::<i64>()
                .map(HoconValue::Int)
                .context("Failed to parse integer")
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_eof() {
            let ch = self.peek();
            if ch.is_whitespace() {
                self.next();
            } else if ch == '#' {
                // Skip comment line
                while !self.is_eof() && self.peek() != '\n' {
                    self.next();
                }
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> char {
        if self.is_eof() {
            '\0'
        } else {
            self.input[self.pos]
        }
    }

    fn next(&mut self) -> char {
        let ch = self.peek();
        if !self.is_eof() {
            self.pos += 1;
        }
        ch
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek() == expected {
            self.next();
            true
        } else {
            false
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

/// Serialize object to HOCON string
fn serialize_object(map: &IndexMap<String, HoconValue>, output: &mut String, indent: usize) {
    for (key, value) in map {
        output.push_str(&"  ".repeat(indent));
        output.push_str(key);
        output.push_str(": ");
        serialize_value(value, output, indent);
        output.push('\n');
    }
}

fn serialize_value(value: &HoconValue, output: &mut String, indent: usize) {
    match value {
        HoconValue::Int(n) => output.push_str(&n.to_string()),
        HoconValue::Float(f) => output.push_str(&f.to_string()),
        HoconValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        HoconValue::String(s) => {
            if s.contains(' ') || s.contains(':') || s.contains('{') || s.contains('[') {
                output.push('"');
                output.push_str(s);
                output.push('"');
            } else {
                output.push_str(s);
            }
        }
        HoconValue::Object(map) => {
            output.push_str("{\n");
            serialize_object(map, output, indent + 1);
            output.push_str(&"  ".repeat(indent));
            output.push('}');
        }
        HoconValue::Array(arr) => {
            output.push_str("[\n");
            for item in arr {
                output.push_str(&"  ".repeat(indent + 1));
                serialize_value(item, output, indent + 1);
                output.push('\n');
            }
            output.push_str(&"  ".repeat(indent));
            output.push(']');
        }
    }
}

impl fmt::Display for HoconValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HoconValue::Int(n) => write!(f, "{}", n),
            HoconValue::Float(fl) => write!(f, "{}", fl),
            HoconValue::Bool(b) => write!(f, "{}", b),
            HoconValue::String(s) => write!(f, "\"{}\"", s),
            HoconValue::Object(_) => write!(f, "{{...}}"),
            HoconValue::Array(arr) => write!(f, "[{} items]", arr.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_values() {
        let hocon = "gems: 21\ngold: 49\nlastCharacter: \"skylar\"";
        let doc = HoconDocument::parse(hocon).unwrap();

        assert_eq!(doc.get("gems"), Some(&HoconValue::Int(21)));
        assert_eq!(doc.get("gold"), Some(&HoconValue::Int(49)));
        assert_eq!(
            doc.get("lastCharacter"),
            Some(&HoconValue::String("skylar".to_string()))
        );
    }

    #[test]
    fn test_parse_object() {
        let hocon = "achievements: { level1: -1 level2: 5 }";
        let doc = HoconDocument::parse(hocon).unwrap();

        if let Some(HoconValue::Object(map)) = doc.get("achievements") {
            assert_eq!(map.get("level1"), Some(&HoconValue::Int(-1)));
            assert_eq!(map.get("level2"), Some(&HoconValue::Int(5)));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_array() {
        let hocon = "perms: [ \"perm1\" \"perm2\" ]";
        let doc = HoconDocument::parse(hocon).unwrap();

        if let Some(HoconValue::Array(arr)) = doc.get("perms") {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0], HoconValue::String("perm1".to_string()));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_roundtrip() {
        let original = "gems: 21\nachievements: { level1: -1 }";
        let doc = HoconDocument::parse(original).unwrap();
        let serialized = doc.to_hocon_string();
        let doc2 = HoconDocument::parse(&serialized).unwrap();

        assert_eq!(doc, doc2);
    }
}
