use std::fs;
use std::path::Path;

use crate::models::{Library, MediaItem, MediaType, Progress, Status};

const DATA_DIR: &str = "data";
const DATA_PATH: &str = "data/library.json";

pub fn load() -> Library {
    if !Path::new(DATA_PATH).exists() {
        return Library::new();
    }

    match fs::read_to_string(DATA_PATH) {
        Ok(text) => parse_library(&text).unwrap_or_else(|err| {
            eprintln!("library.json looks broken ({err}). starting empty.");
            Library::new()
        }),
        Err(_) => {
            eprintln!("couldn't read the log. starting empty.");
            Library::new()
        }
    }
}

pub fn save(library: &Library) {
    if let Err(err) = save_inner(library) {
        eprintln!("couldn't write the log. {err}");
    }
}

fn save_inner(library: &Library) -> Result<(), String> {
    fs::create_dir_all(DATA_DIR).map_err(|e| e.to_string())?;
    fs::write(DATA_PATH, library_to_json(library)).map_err(|e| e.to_string())
}

fn library_to_json(library: &Library) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"next_id\": {},\n", library.next_id()));
    out.push_str("  \"items\": [\n");
    for (i, item) in library.items.iter().enumerate() {
        out.push_str(&item_to_json(item, 4));
        if i + 1 != library.items.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

fn item_to_json(item: &MediaItem, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    let genres = item
        .genres
        .iter()
        .map(|g| format!("\"{}\"", escape_json(g)))
        .collect::<Vec<_>>()
        .join(", ");
    let rating = match item.rating {
        Some(n) => n.to_string(),
        None => "null".to_string(),
    };
    let started = optional_string(&item.date_started);
    let finished = optional_string(&item.date_completed);
    format!(
        "{pad}{{\n\
         {inner}\"id\": {},\n\
         {inner}\"title\": \"{}\",\n\
         {inner}\"media_type\": \"{}\",\n\
         {inner}\"genres\": [{genres}],\n\
         {inner}\"status\": \"{}\",\n\
         {inner}\"progress\": {},\n\
         {inner}\"rating\": {rating},\n\
         {inner}\"date_started\": {started},\n\
         {inner}\"date_completed\": {finished},\n\
         {inner}\"notes\": \"{}\"\n\
         {pad}}}",
        item.id,
        escape_json(&item.title),
        item.media_type.key(),
        item.status.key(),
        progress_to_json(&item.progress),
        escape_json(&item.notes),
    )
}

fn progress_to_json(progress: &Progress) -> String {
    match progress {
        Progress::Pages { current, total } => {
            format!("{{\"kind\": \"Pages\", \"current\": {current}, \"total\": {total}}}")
        }
        Progress::Chapters { current, total } => {
            format!("{{\"kind\": \"Chapters\", \"current\": {current}, \"total\": {total}}}")
        }
        Progress::Episodes { current, total } => {
            format!("{{\"kind\": \"Episodes\", \"current\": {current}, \"total\": {total}}}")
        }
        Progress::MovieWatch { watched } => {
            format!("{{\"kind\": \"MovieWatch\", \"watched\": {watched}}}")
        }
    }
}

fn optional_string(value: &Option<String>) -> String {
    match value {
        Some(s) => format!("\"{}\"", escape_json(s)),
        None => "null".to_string(),
    }
}

fn escape_json(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn object(&self) -> Result<&[(String, Json)], String> {
        match self {
            Json::Object(pairs) => Ok(pairs),
            _ => Err("expected an object.".to_string()),
        }
    }

    fn field(&self, name: &str) -> Result<&Json, String> {
        self.object()?
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
            .ok_or_else(|| format!("missing \"{name}\"."))
    }

    fn as_u32(&self) -> Result<u32, String> {
        match self {
            Json::Number(n) => u32::try_from(*n).map_err(|_| "number too big.".to_string()),
            _ => Err("expected a number.".to_string()),
        }
    }

    fn as_u8(&self) -> Result<u8, String> {
        match self {
            Json::Number(n) => u8::try_from(*n).map_err(|_| "number too big.".to_string()),
            _ => Err("expected a number.".to_string()),
        }
    }

    fn as_bool(&self) -> Result<bool, String> {
        match self {
            Json::Bool(b) => Ok(*b),
            _ => Err("expected true/false.".to_string()),
        }
    }

    fn as_str(&self) -> Result<&str, String> {
        match self {
            Json::String(s) => Ok(s),
            _ => Err("expected a string.".to_string()),
        }
    }

    fn as_array(&self) -> Result<&[Json], String> {
        match self {
            Json::Array(items) => Ok(items),
            _ => Err("expected an array.".to_string()),
        }
    }

    fn optional_string(&self) -> Result<Option<String>, String> {
        match self {
            Json::Null => Ok(None),
            Json::String(s) => Ok(Some(s.clone())),
            _ => Err("expected a string or null.".to_string()),
        }
    }

    fn optional_u8(&self) -> Result<Option<u8>, String> {
        match self {
            Json::Null => Ok(None),
            other => Ok(Some(other.as_u8()?)),
        }
    }
}

struct Parser {
    chars: Vec<char>,
    i: usize,
}

impl Parser {
    fn new(src: &str) -> Self {
        Self {
            chars: src.chars().collect(),
            i: 0,
        }
    }

    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        let c = self.peek().ok_or_else(|| "unexpected end of file.".to_string())?;
        match c {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => Ok(Json::String(self.parse_string()?)),
            't' | 'f' => self.parse_bool(),
            'n' => self.parse_null(),
            '0'..='9' => self.parse_number(),
            _ => Err(format!("unexpected '{c}'.")),
        }
    }

    fn parse_object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.eat('}') {
            return Ok(Json::Object(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip_ws();
            if self.eat('}') {
                break;
            }
            self.expect(',')?;
        }
        Ok(Json::Object(pairs))
    }

    fn parse_array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.eat(']') {
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            if self.eat(']') {
                break;
            }
            self.expect(',')?;
        }
        Ok(Json::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        while let Some(c) = self.next() {
            match c {
                '"' => return Ok(out),
                '\\' => match self.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => return Err("unterminated string.".to_string()),
                },
                other => out.push(other),
            }
        }
        Err("unterminated string.".to_string())
    }

    fn parse_bool(&mut self) -> Result<Json, String> {
        if self.eat_word("true") {
            Ok(Json::Bool(true))
        } else if self.eat_word("false") {
            Ok(Json::Bool(false))
        } else {
            Err("expected true or false.".to_string())
        }
    }

    fn parse_null(&mut self) -> Result<Json, String> {
        if self.eat_word("null") {
            Ok(Json::Null)
        } else {
            Err("expected null.".to_string())
        }
    }

    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.i;
        while matches!(self.peek(), Some('0'..='9')) {
            self.i += 1;
        }
        let raw: String = self.chars[start..self.i].iter().collect();
        let n = raw
            .parse::<u64>()
            .map_err(|_| "bad number.".to_string())?;
        Ok(Json::Number(n))
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += 1;
        Some(c)
    }

    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self, word: &str) -> bool {
        let end = self.i + word.chars().count();
        if end > self.chars.len() {
            return false;
        }
        let got: String = self.chars[self.i..end].iter().collect();
        if got == word {
            self.i = end;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.eat(expected) {
            Ok(())
        } else {
            Err(format!("expected '{expected}'."))
        }
    }
}

fn parse_library(text: &str) -> Result<Library, String> {
    let mut parser = Parser::new(text);
    let root = parser.parse_value()?;
    parser.skip_ws();
    if parser.peek().is_some() {
        return Err("extra text after json.".to_string());
    }
    library_from_json(&root)
}

fn library_from_json(root: &Json) -> Result<Library, String> {
    let next_id = root.field("next_id")?.as_u32()?;
    let mut items = Vec::new();
    for value in root.field("items")?.as_array()? {
        items.push(item_from_json(value)?);
    }
    Ok(Library::from_saved(next_id, items))
}

fn item_from_json(value: &Json) -> Result<MediaItem, String> {
    let media_type = MediaType::from_key(value.field("media_type")?.as_str()?)
        .ok_or_else(|| "unknown media type.".to_string())?;
    let status = Status::from_key(value.field("status")?.as_str()?)
        .ok_or_else(|| "unknown status.".to_string())?;
    let genres = value
        .field("genres")?
        .as_array()?
        .iter()
        .map(|g| g.as_str().map(|s| s.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MediaItem {
        id: value.field("id")?.as_u32()?,
        title: value.field("title")?.as_str()?.to_string(),
        media_type,
        genres,
        status,
        progress: progress_from_json(value.field("progress")?)?,
        rating: value.field("rating")?.optional_u8()?,
        date_started: value.field("date_started")?.optional_string()?,
        date_completed: value.field("date_completed")?.optional_string()?,
        notes: value.field("notes")?.as_str()?.to_string(),
    })
}

fn progress_from_json(value: &Json) -> Result<Progress, String> {
    match value.field("kind")?.as_str()? {
        "Pages" => Ok(Progress::Pages {
            current: value.field("current")?.as_u32()?,
            total: value.field("total")?.as_u32()?,
        }),
        "Chapters" => Ok(Progress::Chapters {
            current: value.field("current")?.as_u32()?,
            total: value.field("total")?.as_u32()?,
        }),
        "Episodes" => Ok(Progress::Episodes {
            current: value.field("current")?.as_u32()?,
            total: value.field("total")?.as_u32()?,
        }),
        "MovieWatch" => Ok(Progress::MovieWatch {
            watched: value.field("watched")?.as_bool()?,
        }),
        _ => Err("unknown progress kind.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_keeps_fields() {
        let mut library = Library::new();
        library.add(MediaItem {
            id: 0,
            title: "Solo \"Leveling\"".into(),
            media_type: MediaType::Webtoon,
            genres: vec!["Action".into(), "Fantasy".into()],
            status: Status::WantToConsume,
            progress: Progress::Episodes {
                current: 8,
                total: 24,
            },
            rating: Some(5),
            date_started: None,
            date_completed: None,
            notes: "night read".into(),
        });
        let json = library_to_json(&library);
        let loaded = parse_library(&json).unwrap();
        assert_eq!(loaded.next_id(), 2);
        assert_eq!(loaded.items[0].title, "Solo \"Leveling\"");
        assert_eq!(loaded.items[0].rating, Some(5));
        assert_eq!(
            loaded.items[0].progress,
            Progress::Episodes {
                current: 8,
                total: 24
            }
        );
    }
}
