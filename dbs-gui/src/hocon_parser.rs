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

    #[test]
    fn test_parse_real_savegame() {
        // Real savegame structure from data/saves/save.hocon
        let hocon = r#"
challengeDate: 20251103
gameMode: "normal"
gems: 4379
gemsTotal: 85
gold: 0
highestLevel: 6
lastCharacter: "skylar"
permsVersion: 3
personalBestGold: 1408
replayCode: "KDLCTJ"
statsDay: 20251103
statsVersion: 1
version: 1
welcomeGems: 25
achievements: {
    challenges1: -1
    challenges10: 1
    challenges100: 1
    kill1000: 253
    kill500: 253
    level1: -1
    level3: -1
    level5: -1
    stump_inspector: 1
}
challenges: {
    chainsaw25: 0
    killzombies10: 0
    level4jacklyn: 0
}
perms: [
    "perm_extraheart1"
    "perm_gold_saver"
]
highestLevels: {
    jacklyn: 4
    skylar: 6
}
totalStats: {
    bombs: 35
    died: 2
    dist_ran: 1555
    gems: 54
    gold: 1676
    max_level: 7
    biomes_visited: {
        desert: 5
        grass: 5
        snow: 1
    }
}
"#;

        let doc = HoconDocument::parse(hocon).unwrap();

        // Test top-level integer values
        assert_eq!(doc.get("gems"), Some(&HoconValue::Int(4379)));
        assert_eq!(doc.get("gemsTotal"), Some(&HoconValue::Int(85)));
        assert_eq!(doc.get("gold"), Some(&HoconValue::Int(0)));
        assert_eq!(doc.get("highestLevel"), Some(&HoconValue::Int(6)));
        assert_eq!(doc.get("version"), Some(&HoconValue::Int(1)));

        // Test string values
        assert_eq!(
            doc.get("gameMode"),
            Some(&HoconValue::String("normal".to_string()))
        );
        assert_eq!(
            doc.get("lastCharacter"),
            Some(&HoconValue::String("skylar".to_string()))
        );
        assert_eq!(
            doc.get("replayCode"),
            Some(&HoconValue::String("KDLCTJ".to_string()))
        );

        // Test nested object (achievements)
        if let Some(HoconValue::Object(achievements)) = doc.get("achievements") {
            assert_eq!(achievements.get("challenges1"), Some(&HoconValue::Int(-1)));
            assert_eq!(achievements.get("challenges10"), Some(&HoconValue::Int(1)));
            assert_eq!(achievements.get("kill1000"), Some(&HoconValue::Int(253)));
            assert_eq!(achievements.get("level1"), Some(&HoconValue::Int(-1)));
            assert_eq!(achievements.get("stump_inspector"), Some(&HoconValue::Int(1)));
        } else {
            panic!("Expected achievements object");
        }

        // Test another nested object (challenges)
        if let Some(HoconValue::Object(challenges)) = doc.get("challenges") {
            assert_eq!(challenges.get("chainsaw25"), Some(&HoconValue::Int(0)));
            assert_eq!(challenges.get("killzombies10"), Some(&HoconValue::Int(0)));
            assert_eq!(challenges.get("level4jacklyn"), Some(&HoconValue::Int(0)));
        } else {
            panic!("Expected challenges object");
        }

        // Test string array
        if let Some(HoconValue::Array(perms)) = doc.get("perms") {
            assert_eq!(perms.len(), 2);
            assert_eq!(perms[0], HoconValue::String("perm_extraheart1".to_string()));
            assert_eq!(perms[1], HoconValue::String("perm_gold_saver".to_string()));
        } else {
            panic!("Expected perms array");
        }

        // Test nested object (highestLevels)
        if let Some(HoconValue::Object(highest_levels)) = doc.get("highestLevels") {
            assert_eq!(highest_levels.get("jacklyn"), Some(&HoconValue::Int(4)));
            assert_eq!(highest_levels.get("skylar"), Some(&HoconValue::Int(6)));
        } else {
            panic!("Expected highestLevels object");
        }

        // Test deeply nested object (totalStats with biomes_visited)
        if let Some(HoconValue::Object(total_stats)) = doc.get("totalStats") {
            assert_eq!(total_stats.get("bombs"), Some(&HoconValue::Int(35)));
            assert_eq!(total_stats.get("died"), Some(&HoconValue::Int(2)));
            assert_eq!(total_stats.get("dist_ran"), Some(&HoconValue::Int(1555)));
            assert_eq!(total_stats.get("gems"), Some(&HoconValue::Int(54)));
            assert_eq!(total_stats.get("max_level"), Some(&HoconValue::Int(7)));

            // Test 3-level deep nesting
            if let Some(HoconValue::Object(biomes)) = total_stats.get("biomes_visited") {
                assert_eq!(biomes.get("desert"), Some(&HoconValue::Int(5)));
                assert_eq!(biomes.get("grass"), Some(&HoconValue::Int(5)));
                assert_eq!(biomes.get("snow"), Some(&HoconValue::Int(1)));
            } else {
                panic!("Expected biomes_visited object");
            }
        } else {
            panic!("Expected totalStats object");
        }
    }

    #[test]
    fn test_parse_real_savegame_with_complex_arrays() {
        // Test object arrays like highscores and leaderboards
        let hocon = r#"
highscores: {
    version: 2
    bombs_exploded: [
        {
            extra2: 8071998
            extra3: 1761852850
            level: 6
            player: "skylar"
            score: 35
        }
        {
            extra2: 12206436
            extra3: 1761852935
            level: 7
            player: "skylar"
            score: 35
        }
    ]
    distance_ran: [
        {
            extra2: 10434534
            extra3: 1762210886
            level: 1
            player: "skylar"
            score: 1555
        }
    ]
}
completeChallenges: [
    20251030
    "/reaperfreeze5"
]
"#;

        let doc = HoconDocument::parse(hocon).unwrap();

        // Test object with arrays of objects
        if let Some(HoconValue::Object(highscores)) = doc.get("highscores") {
            assert_eq!(highscores.get("version"), Some(&HoconValue::Int(2)));

            // Test array of objects (bombs_exploded)
            if let Some(HoconValue::Array(bombs)) = highscores.get("bombs_exploded") {
                assert_eq!(bombs.len(), 2);

                // Check first object in array
                if let HoconValue::Object(first_bomb) = &bombs[0] {
                    assert_eq!(first_bomb.get("extra2"), Some(&HoconValue::Int(8071998)));
                    assert_eq!(first_bomb.get("level"), Some(&HoconValue::Int(6)));
                    assert_eq!(
                        first_bomb.get("player"),
                        Some(&HoconValue::String("skylar".to_string()))
                    );
                    assert_eq!(first_bomb.get("score"), Some(&HoconValue::Int(35)));
                } else {
                    panic!("Expected object in bombs_exploded array");
                }

                // Check second object
                if let HoconValue::Object(second_bomb) = &bombs[1] {
                    assert_eq!(second_bomb.get("level"), Some(&HoconValue::Int(7)));
                } else {
                    panic!("Expected second object in bombs_exploded array");
                }
            } else {
                panic!("Expected bombs_exploded array");
            }

            // Test another array of objects (distance_ran)
            if let Some(HoconValue::Array(distance)) = highscores.get("distance_ran") {
                assert_eq!(distance.len(), 1);
                if let HoconValue::Object(first_dist) = &distance[0] {
                    assert_eq!(first_dist.get("score"), Some(&HoconValue::Int(1555)));
                } else {
                    panic!("Expected object in distance_ran array");
                }
            } else {
                panic!("Expected distance_ran array");
            }
        } else {
            panic!("Expected highscores object");
        }

        // Test mixed array (int and string)
        if let Some(HoconValue::Array(challenges)) = doc.get("completeChallenges") {
            assert_eq!(challenges.len(), 2);
            assert_eq!(challenges[0], HoconValue::Int(20251030));
            assert_eq!(
                challenges[1],
                HoconValue::String("/reaperfreeze5".to_string())
            );
        } else {
            panic!("Expected completeChallenges array");
        }
    }

    #[test]
    fn test_real_savegame_roundtrip() {
        // Test that we can parse and re-serialize a complex real savegame
        let hocon = r#"
gems: 4379
gameMode: "normal"
achievements: {
    kill1000: 253
    level1: -1
}
perms: [
    "perm_extraheart1"
    "perm_gold_saver"
]
highscores: {
    version: 2
    bombs_exploded: [
        {
            level: 6
            player: "skylar"
            score: 35
        }
    ]
}
"#;

        let doc = HoconDocument::parse(hocon).unwrap();
        let serialized = doc.to_hocon_string();
        let doc2 = HoconDocument::parse(&serialized).unwrap();

        // Verify the roundtrip preserved all data
        assert_eq!(doc2.get("gems"), Some(&HoconValue::Int(4379)));
        assert_eq!(
            doc2.get("gameMode"),
            Some(&HoconValue::String("normal".to_string()))
        );

        // Verify nested objects
        if let Some(HoconValue::Object(achievements)) = doc2.get("achievements") {
            assert_eq!(achievements.get("kill1000"), Some(&HoconValue::Int(253)));
            assert_eq!(achievements.get("level1"), Some(&HoconValue::Int(-1)));
        } else {
            panic!("Expected achievements after roundtrip");
        }

        // Verify arrays
        if let Some(HoconValue::Array(perms)) = doc2.get("perms") {
            assert_eq!(perms.len(), 2);
        } else {
            panic!("Expected perms array after roundtrip");
        }
    }
}
