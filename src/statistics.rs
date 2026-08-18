use crate::models::{MediaItem, MediaType, Status};

#[derive(Debug, PartialEq)]
pub struct Stats {
    pub total: usize,
    pub completed: usize,
    pub in_progress: usize,
    pub completed_book: usize,
    pub completed_manga: usize,
    pub completed_webtoon: usize,
    pub completed_movie: usize,
    pub completed_tv: usize,
    pub completed_anime: usize,
    pub average_rating: Option<f32>,
}

pub fn compute(items: &[MediaItem]) -> Stats {
    let done = |t: MediaType| {
        items
            .iter()
            .filter(|i| i.status == Status::Completed && i.media_type == t)
            .count()
    };

    let ratings: Vec<u8> = items.iter().filter_map(|i| i.rating).collect();
    let average_rating = if ratings.is_empty() {
        None
    } else {
        let sum: u32 = ratings.iter().map(|&r| u32::from(r)).sum();
        Some(sum as f32 / ratings.len() as f32)
    };

    Stats {
        total: items.len(),
        completed: items
            .iter()
            .filter(|i| i.status == Status::Completed)
            .count(),
        in_progress: items
            .iter()
            .filter(|i| i.status == Status::InProgress)
            .count(),
        completed_book: done(MediaType::Book),
        completed_manga: done(MediaType::Manga),
        completed_webtoon: done(MediaType::Webtoon),
        completed_movie: done(MediaType::Movie),
        completed_tv: done(MediaType::TvSeries),
        completed_anime: done(MediaType::Anime),
        average_rating,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Progress, Status};

    fn item(media_type: MediaType, status: Status, rating: Option<u8>) -> MediaItem {
        let progress = match media_type {
            MediaType::Book => Progress::Pages {
                current: 0,
                total: 0,
            },
            MediaType::Manga => Progress::Chapters {
                current: 0,
                total: 0,
            },
            MediaType::Webtoon | MediaType::TvSeries | MediaType::Anime => Progress::Episodes {
                current: 0,
                total: 0,
            },
            MediaType::Movie => Progress::MovieWatch { watched: false },
        };
        MediaItem {
            id: 1,
            title: "x".into(),
            media_type,
            genres: Vec::new(),
            status,
            progress,
            rating,
            date_started: None,
            date_completed: None,
            notes: String::new(),
        }
    }

    #[test]
    fn counts_completed_per_type() {
        let items = vec![
            item(MediaType::Book, Status::Completed, Some(4)),
            item(MediaType::Book, Status::InProgress, Some(5)),
            item(MediaType::Anime, Status::Completed, Some(3)),
            item(MediaType::Manga, Status::WantToConsume, None),
        ];
        let stats = compute(&items);
        assert_eq!(stats.total, 4);
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.in_progress, 1);
        assert_eq!(stats.completed_book, 1);
        assert_eq!(stats.completed_anime, 1);
        assert_eq!(stats.completed_manga, 0);
        assert_eq!(stats.average_rating, Some(4.0));
    }

    #[test]
    fn average_is_none_without_ratings() {
        let items = vec![item(MediaType::Movie, Status::WantToConsume, None)];
        assert_eq!(compute(&items).average_rating, None);
    }
}
