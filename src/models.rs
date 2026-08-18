use std::collections::HashMap;

/// Kind of media on the shelf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Book,
    Manga,
    Webtoon,
    Movie,
    TvSeries,
    Anime,
}

impl MediaType {
    pub fn label(self) -> &'static str {
        match self {
            MediaType::Book => "book",
            MediaType::Manga => "manga",
            MediaType::Webtoon => "webtoon",
            MediaType::Movie => "movie",
            MediaType::TvSeries => "tv",
            MediaType::Anime => "anime",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            MediaType::Book => "Book",
            MediaType::Manga => "Manga",
            MediaType::Webtoon => "Webtoon",
            MediaType::Movie => "Movie",
            MediaType::TvSeries => "TvSeries",
            MediaType::Anime => "Anime",
        }
    }

    pub fn from_key(raw: &str) -> Option<Self> {
        match raw {
            "Book" => Some(MediaType::Book),
            "Manga" => Some(MediaType::Manga),
            "Webtoon" => Some(MediaType::Webtoon),
            "Movie" => Some(MediaType::Movie),
            "TvSeries" => Some(MediaType::TvSeries),
            "Anime" => Some(MediaType::Anime),
            _ => None,
        }
    }
}

/// Where an item sits in the pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    WantToConsume,
    InProgress,
    Completed,
    OnHold,
    Dropped,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::WantToConsume => "want",
            Status::InProgress => "now",
            Status::Completed => "done",
            Status::OnHold => "hold",
            Status::Dropped => "dropped",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Status::WantToConsume => "WantToConsume",
            Status::InProgress => "InProgress",
            Status::Completed => "Completed",
            Status::OnHold => "OnHold",
            Status::Dropped => "Dropped",
        }
    }

    pub fn from_key(raw: &str) -> Option<Self> {
        match raw {
            "WantToConsume" => Some(Status::WantToConsume),
            "InProgress" => Some(Status::InProgress),
            "Completed" => Some(Status::Completed),
            "OnHold" => Some(Status::OnHold),
            "Dropped" => Some(Status::Dropped),
            _ => None,
        }
    }
}

/// Progress depends on the media type (pattern matching).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    Pages { current: u32, total: u32 },
    Chapters { current: u32, total: u32 },
    Episodes { current: u32, total: u32 },
    MovieWatch { watched: bool },
}

impl Progress {
    pub fn display(&self) -> String {
        match self {
            Progress::Pages { current, total } => format!("{current} / {total} pages"),
            Progress::Chapters { current, total } => format!("{current} / {total} chapters"),
            Progress::Episodes { current, total } => format!("{current} / {total} episodes"),
            Progress::MovieWatch { watched: true } => "completed".to_string(),
            Progress::MovieWatch { watched: false } => "not completed".to_string(),
        }
    }

    pub fn counts(&self) -> Option<(u32, u32)> {
        match self {
            Progress::Pages { current, total }
            | Progress::Chapters { current, total }
            | Progress::Episodes { current, total } => Some((*current, *total)),
            Progress::MovieWatch { .. } => None,
        }
    }

    pub fn set_counts(&mut self, current: u32, total: u32) -> Result<(), String> {
        if current > total {
            return Err("current can't be past the total.".to_string());
        }
        match self {
            Progress::Pages {
                current: c,
                total: t,
            }
            | Progress::Chapters {
                current: c,
                total: t,
            }
            | Progress::Episodes {
                current: c,
                total: t,
            } => {
                *c = current;
                *t = total;
                Ok(())
            }
            Progress::MovieWatch { .. } => Err("movies don't use page counts.".to_string()),
        }
    }

    pub fn set_watched(&mut self, watched: bool) -> Result<(), String> {
        match self {
            Progress::MovieWatch { watched: w } => {
                *w = watched;
                Ok(())
            }
            _ => Err("only movies use watched / not.".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub id: u32,
    pub title: String,
    pub media_type: MediaType,
    pub genres: Vec<String>,
    pub status: Status,
    pub progress: Progress,
    pub rating: Option<u8>,
    pub date_started: Option<String>,
    pub date_completed: Option<String>,
    pub notes: String,
}

impl MediaItem {
    pub fn genre_line(&self) -> String {
        self.genres.join(", ")
    }

    pub fn set_rating(&mut self, rating: u8) -> Result<(), String> {
        validate_rating(rating)?;
        self.rating = Some(rating);
        Ok(())
    }

    /// Marking something done also fills in progress when we already know the total.
    pub fn apply_status(&mut self, status: Status) {
        self.status = status;
        if status != Status::Completed {
            return;
        }
        match &mut self.progress {
            Progress::Pages { current, total }
            | Progress::Chapters { current, total }
            | Progress::Episodes { current, total }
                if *total > 0 =>
            {
                *current = *total;
            }
            Progress::MovieWatch { watched } => *watched = true,
            _ => {}
        }
    }
}

#[derive(Debug)]
pub struct Library {
    next_id: u32,
    pub items: Vec<MediaItem>,
}

impl Library {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
        }
    }

    pub fn from_saved(next_id: u32, items: Vec<MediaItem>) -> Self {
        Self { next_id, items }
    }

    pub fn next_id(&self) -> u32 {
        self.next_id
    }

    pub fn add(&mut self, mut item: MediaItem) -> u32 {
        let id = self.next_id;
        item.id = id;
        self.next_id += 1;
        self.items.push(item);
        id
    }

    /// HashMap from id -> index, used for get / remove.
    fn index_map(&self) -> HashMap<u32, usize> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| (item.id, i))
            .collect()
    }

    pub fn find_index(&self, id: u32) -> Option<usize> {
        self.index_map().get(&id).copied()
    }

    pub fn get(&self, id: u32) -> Option<&MediaItem> {
        let idx = self.find_index(id)?;
        self.items.get(idx)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut MediaItem> {
        let idx = self.find_index(id)?;
        self.items.get_mut(idx)
    }

    pub fn remove(&mut self, id: u32) -> Option<MediaItem> {
        let idx = self.find_index(id)?;
        Some(self.items.remove(idx))
    }
}

pub fn validate_rating(rating: u8) -> Result<u8, String> {
    if (1..=5).contains(&rating) {
        Ok(rating)
    } else {
        Err("rating has to be 1-5.".to_string())
    }
}

pub fn parse_rating(raw: &str) -> Result<u8, String> {
    let n: u8 = raw
        .trim()
        .parse()
        .map_err(|_| "rating has to be 1-5.".to_string())?;
    validate_rating(n)
}

pub fn parse_u32(raw: &str) -> Result<u32, String> {
    raw.trim()
        .parse::<u32>()
        .map_err(|_| "need a whole number.".to_string())
}

pub fn parse_optional_date(raw: &str) -> Result<Option<String>, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let parts: Vec<&str> = s.split('-').collect();
    let ok = parts.len() == 3
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()));
    if ok {
        Ok(Some(s.to_string()))
    } else {
        Err("use YYYY-MM-DD, or leave blank.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rating_rejects_out_of_range() {
        assert!(parse_rating("0").is_err());
        assert!(parse_rating("6").is_err());
        assert!(parse_rating("nope").is_err());
    }

    #[test]
    fn rating_accepts_one_through_five() {
        assert_eq!(parse_rating("1").unwrap(), 1);
        assert_eq!(parse_rating("5").unwrap(), 5);
    }

    #[test]
    fn progress_display_matches_type() {
        assert_eq!(
            Progress::Pages {
                current: 125,
                total: 300
            }
            .display(),
            "125 / 300 pages"
        );
        assert_eq!(
            Progress::Chapters {
                current: 45,
                total: 100
            }
            .display(),
            "45 / 100 chapters"
        );
        assert_eq!(
            Progress::Episodes {
                current: 8,
                total: 24
            }
            .display(),
            "8 / 24 episodes"
        );
        assert_eq!(
            Progress::MovieWatch { watched: false }.display(),
            "not completed"
        );
    }

    #[test]
    fn progress_rejects_current_past_total() {
        let mut p = Progress::Pages {
            current: 0,
            total: 10,
        };
        assert!(p.set_counts(11, 10).is_err());
        assert!(p.set_counts(10, 10).is_ok());
    }

    #[test]
    fn library_lookup_uses_id() {
        let mut lib = Library::new();
        let id = lib.add(MediaItem {
            id: 0,
            title: "Solo Leveling".into(),
            media_type: MediaType::Webtoon,
            genres: vec!["Action".into()],
            status: Status::WantToConsume,
            progress: Progress::Episodes {
                current: 0,
                total: 0,
            },
            rating: None,
            date_started: None,
            date_completed: None,
            notes: String::new(),
        });
        assert_eq!(id, 1);
        assert_eq!(lib.get(1).unwrap().title, "Solo Leveling");
        assert!(lib.get(99).is_none());
    }
}
