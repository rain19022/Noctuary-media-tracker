use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{MediaItem, MediaType, Status};

pub fn search_title<'a>(items: &'a [MediaItem], query: &str) -> Vec<&'a MediaItem> {
    if query.trim().is_empty() {
        return items.iter().collect();
    }
    let q = query.to_lowercase();
    items
        .iter()
        .filter(|i| i.title.to_lowercase().contains(&q))
        .collect()
}

pub fn pick_random<'a>(items: &'a [MediaItem]) -> Option<&'a MediaItem> {
    let want: Vec<&MediaItem> = items
        .iter()
        .filter(|i| i.status == Status::WantToConsume)
        .collect();
    if want.is_empty() {
        return None;
    }
    let idx = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as usize % want.len())
        .unwrap_or(0);
    Some(want[idx])
}

pub struct FindFilters<'a> {
    pub title: &'a str,
    pub media_type: Option<MediaType>,
    pub status: Option<Status>,
    pub genre: &'a str,
    pub sort_by_rating: bool,
}

pub fn apply_filters<'a>(items: &'a [MediaItem], filters: &FindFilters<'_>) -> Vec<&'a MediaItem> {
    let mut list: Vec<&MediaItem> = if filters.title.trim().is_empty() {
        items.iter().collect()
    } else {
        search_title(items, filters.title)
    };

    if let Some(t) = filters.media_type {
        list.retain(|i| i.media_type == t);
    }
    if let Some(s) = filters.status {
        list.retain(|i| i.status == s);
    }
    if !filters.genre.trim().is_empty() {
        let genre = filters.genre.to_lowercase();
        list.retain(|i| i.genres.iter().any(|g| g.to_lowercase().contains(&genre)));
    }
    if filters.sort_by_rating {
        list.sort_by(|a, b| match (a.rating, b.rating) {
            (Some(x), Some(y)) => y.cmp(&x),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        });
    }

    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Progress;

    fn sample() -> Vec<MediaItem> {
        vec![
            MediaItem {
                id: 1,
                title: "Solo Leveling".into(),
                media_type: MediaType::Webtoon,
                genres: vec!["Action".into(), "Fantasy".into()],
                status: Status::WantToConsume,
                progress: Progress::Episodes {
                    current: 0,
                    total: 0,
                },
                rating: Some(5),
                date_started: None,
                date_completed: None,
                notes: String::new(),
            },
            MediaItem {
                id: 2,
                title: "Dune".into(),
                media_type: MediaType::Book,
                genres: vec!["Sci-Fi".into()],
                status: Status::Completed,
                progress: Progress::Pages {
                    current: 800,
                    total: 800,
                },
                rating: Some(4),
                date_started: None,
                date_completed: None,
                notes: String::new(),
            },
        ]
    }

    #[test]
    fn title_search_is_case_insensitive() {
        let items = sample();
        let found = search_title(&items, "solo");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "Solo Leveling");
    }

    #[test]
    fn genre_filter_matches_partial() {
        let items = sample();
        let filters = FindFilters {
            title: "",
            media_type: None,
            status: None,
            genre: "sci",
            sort_by_rating: false,
        };
        assert_eq!(apply_filters(&items, &filters).len(), 1);
    }

    #[test]
    fn rating_sort_puts_highest_first() {
        let items = sample();
        let filters = FindFilters {
            title: "",
            media_type: None,
            status: None,
            genre: "",
            sort_by_rating: true,
        };
        let sorted = apply_filters(&items, &filters);
        assert_eq!(sorted[0].title, "Solo Leveling");
        assert_eq!(sorted[1].title, "Dune");
    }
}
