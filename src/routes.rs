use std::sync::{Arc, Mutex};

use crate::media::{self, FindFilters};
use crate::models::{
    Library, MediaItem, MediaType, Progress, Status, parse_optional_date, parse_rating, parse_u32,
};
use crate::pages;
use crate::server::{HttpRequest, HttpResponse};
use crate::statistics;
use crate::storage;

pub struct AppState {
    pub library: Mutex<Library>,
}

pub fn handle(state: &Arc<AppState>, request: &HttpRequest) -> HttpResponse {
    let path = &request.path;

    match (request.method.as_str(), path.as_str()) {
        ("GET", "/") => library_get(state, request),
        ("GET", "/add") => HttpResponse::html(pages::add_form(None)),
        ("POST", "/add") => add_post(state, request),
        ("GET", "/find") => find_get(state, request),
        ("GET", "/numbers") => numbers_get(state),
        ("GET", "/pick") => pick_get(state),
        ("GET", p) if p.starts_with("/static/") => serve_static(p),
        ("GET", p) if p.starts_with("/item/") => item_get(state, p, request),
        ("POST", p) if p.ends_with("/progress") => item_progress(state, p, request),
        ("POST", p) if p.ends_with("/status") => item_status(state, p, request),
        ("POST", p) if p.ends_with("/rating") => item_rating(state, p, request),
        ("POST", p) if p.ends_with("/delete") => item_delete(state, p),
        _ => HttpResponse::not_found(),
    }
}

fn save(state: &AppState) {
    let lib = state.library.lock().unwrap();
    storage::save(&lib);
}

fn flash(request: &HttpRequest) -> Option<String> {
    request.query.get("flash").cloned()
}

fn flash_url(path: &str, msg: &str) -> String {
    let encoded: String = msg
        .chars()
        .map(|c| match c {
            ' ' => "%20".into(),
            ':' => "%3A".into(),
            _ if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect();
    format!("{path}?flash={encoded}")
}

fn parse_id(path: &str) -> Option<u32> {
    let rest = path.strip_prefix("/item/")?;
    rest.split('/').next()?.parse().ok()
}

fn library_get(state: &Arc<AppState>, request: &HttpRequest) -> HttpResponse {
    let lib = state.library.lock().unwrap();
    HttpResponse::html(pages::library(&lib.items, flash(request).as_deref()))
}

fn add_post(state: &Arc<AppState>, request: &HttpRequest) -> HttpResponse {
    let title = request.form.get("title").map(|s| s.trim()).unwrap_or("");
    if title.is_empty() {
        return HttpResponse::html(pages::add_form(Some("title can't be empty.")));
    }

    let media_type = match request
        .form
        .get("media_type")
        .and_then(|s| MediaType::from_key(s))
    {
        Some(t) => t,
        None => return HttpResponse::html(pages::add_form(Some("pick a type."))),
    };
    let status = match request.form.get("status").and_then(|s| Status::from_key(s)) {
        Some(s) => s,
        None => return HttpResponse::html(pages::add_form(Some("pick a status."))),
    };

    let current = request
        .form
        .get("current")
        .map(|s| parse_u32(s).unwrap_or(0))
        .unwrap_or(0);
    let total = request
        .form
        .get("total")
        .map(|s| parse_u32(s).unwrap_or(0))
        .unwrap_or(0);
    let watched = request.form.contains_key("watched");

    let progress = match build_progress(media_type, status, current, total, watched) {
        Ok(p) => p,
        Err(msg) => return HttpResponse::html(pages::add_form(Some(&msg))),
    };

    let rating = match request.form.get("rating").map(|s| s.as_str()).unwrap_or("").trim() {
        "" => None,
        raw => match parse_rating(raw) {
            Ok(n) => Some(n),
            Err(msg) => return HttpResponse::html(pages::add_form(Some(&msg))),
        },
    };

    let date_started = match parse_optional_date(request.form.get("date_started").map(|s| s.as_str()).unwrap_or("")) {
        Ok(d) => d,
        Err(msg) => return HttpResponse::html(pages::add_form(Some(&msg))),
    };
    let date_completed =
        match parse_optional_date(request.form.get("date_completed").map(|s| s.as_str()).unwrap_or("")) {
            Ok(d) => d,
            Err(msg) => return HttpResponse::html(pages::add_form(Some(&msg))),
        };

    let genres: Vec<String> = request
        .form
        .get("genres")
        .map(|s| {
            s.split(',')
                .map(|g| g.trim().to_string())
                .filter(|g| !g.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut lib = state.library.lock().unwrap();
    let id = lib.add(MediaItem {
        id: 0,
        title: title.to_string(),
        media_type,
        genres,
        status,
        progress,
        rating,
        date_started,
        date_completed,
        notes: request
            .form
            .get("notes")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
    });
    drop(lib);
    save(state);

    HttpResponse::redirect(&flash_url("/", &format!("in the noctuary. id {id}")))
}

fn find_get(state: &Arc<AppState>, request: &HttpRequest) -> HttpResponse {
    let title = request.query.get("title").cloned().unwrap_or_default();
    let genre = request.query.get("genre").cloned().unwrap_or_default();
    let media_type = request
        .query
        .get("media_type")
        .and_then(|s| MediaType::from_key(s));
    let status = request.query.get("status").and_then(|s| Status::from_key(s));
    let sort_by_rating = request.query.get("sort").map(|s| s.as_str()) == Some("rating");

    let lib = state.library.lock().unwrap();
    let filters = FindFilters {
        title: &title,
        media_type,
        status,
        genre: &genre,
        sort_by_rating,
    };
    let results = media::apply_filters(&lib.items, &filters);
    HttpResponse::html(pages::find_page(
        &results,
        &title,
        media_type,
        status,
        &genre,
        sort_by_rating,
    ))
}

fn item_get(state: &Arc<AppState>, path: &str, request: &HttpRequest) -> HttpResponse {
    let Some(id) = parse_id(path) else {
        return HttpResponse::html(pages::not_found());
    };
    let lib = state.library.lock().unwrap();
    let Some(item) = lib.get(id) else {
        return HttpResponse::html(pages::not_found());
    };
    HttpResponse::html(pages::item_detail(item, flash(request).as_deref()))
}

fn item_progress(state: &Arc<AppState>, path: &str, request: &HttpRequest) -> HttpResponse {
    let Some(id) = parse_id(path) else {
        return HttpResponse::html(pages::not_found());
    };

    let mut lib = state.library.lock().unwrap();
    let Some(item) = lib.get_mut(id) else {
        return HttpResponse::html(pages::not_found());
    };

    let err = match &item.progress {
        Progress::MovieWatch { .. } => {
            let watched = request.form.contains_key("watched");
            item.progress.set_watched(watched).err()
        }
        _ => {
            let current = request
                .form
                .get("current")
                .map(|s| parse_u32(s).unwrap_or(0))
                .unwrap_or(0);
            let total = request
                .form
                .get("total")
                .map(|s| parse_u32(s).unwrap_or(0))
                .unwrap_or(0);
            item.progress.set_counts(current, total).err()
        }
    };

    if let Some(msg) = err {
        return HttpResponse::html(pages::item_detail(item, Some(&msg)));
    }

    drop(lib);
    save(state);
    HttpResponse::redirect(&format!("/item/{id}"))
}

fn item_status(state: &Arc<AppState>, path: &str, request: &HttpRequest) -> HttpResponse {
    let Some(id) = parse_id(path) else {
        return HttpResponse::html(pages::not_found());
    };

    let mut lib = state.library.lock().unwrap();
    let Some(item) = lib.get_mut(id) else {
        return HttpResponse::html(pages::not_found());
    };
    if let Some(status) = request.form.get("status").and_then(|s| Status::from_key(s)) {
        item.apply_status(status);
    }
    drop(lib);
    save(state);
    HttpResponse::redirect(&format!("/item/{id}"))
}

fn item_rating(state: &Arc<AppState>, path: &str, request: &HttpRequest) -> HttpResponse {
    let Some(id) = parse_id(path) else {
        return HttpResponse::html(pages::not_found());
    };

    let mut lib = state.library.lock().unwrap();
    let Some(item) = lib.get_mut(id) else {
        return HttpResponse::html(pages::not_found());
    };

    match parse_rating(request.form.get("rating").map(|s| s.as_str()).unwrap_or("")) {
        Ok(n) => {
            if let Err(msg) = item.set_rating(n) {
                return HttpResponse::html(pages::item_detail(item, Some(&msg)));
            }
        }
        Err(msg) => return HttpResponse::html(pages::item_detail(item, Some(&msg))),
    }

    drop(lib);
    save(state);
    HttpResponse::redirect(&format!("/item/{id}"))
}

fn item_delete(state: &Arc<AppState>, path: &str) -> HttpResponse {
    let Some(id) = parse_id(path) else {
        return HttpResponse::html(pages::not_found());
    };

    let mut lib = state.library.lock().unwrap();
    let Some(item) = lib.remove(id) else {
        return HttpResponse::html(pages::not_found());
    };
    let title = item.title.clone();
    drop(lib);
    save(state);
    HttpResponse::redirect(&flash_url("/", &format!("removed: {title}")))
}

fn numbers_get(state: &Arc<AppState>) -> HttpResponse {
    let lib = state.library.lock().unwrap();
    HttpResponse::html(pages::numbers(&statistics::compute(&lib.items)))
}

fn pick_get(state: &Arc<AppState>) -> HttpResponse {
    let lib = state.library.lock().unwrap();
    HttpResponse::html(pages::pick(media::pick_random(&lib.items)))
}

fn serve_static(path: &str) -> HttpResponse {
    let file = path.strip_prefix("/static/").unwrap_or("");
    if file.is_empty() || file.contains("..") {
        return HttpResponse::not_found();
    }
    let full = format!("static/{file}");
    match std::fs::read(&full) {
        Ok(data) => {
            let ct = if file.ends_with(".css") {
                "text/css"
            } else {
                "application/octet-stream"
            };
            HttpResponse::static_file(ct, data)
        }
        Err(_) => HttpResponse::not_found(),
    }
}

fn build_progress(
    media_type: MediaType,
    status: Status,
    current: u32,
    total: u32,
    watched: bool,
) -> Result<Progress, String> {
    match media_type {
        MediaType::Movie => Ok(Progress::MovieWatch {
            watched: watched || status == Status::Completed,
        }),
        MediaType::Book => {
            let mut p = Progress::Pages { current, total };
            if current > total && total > 0 {
                return Err("current can't be past the total.".to_string());
            }
            if status == Status::Completed && total > 0 {
                p.set_counts(total, total)?;
            }
            Ok(p)
        }
        MediaType::Manga => {
            let mut p = Progress::Chapters { current, total };
            if current > total && total > 0 {
                return Err("current can't be past the total.".to_string());
            }
            if status == Status::Completed && total > 0 {
                p.set_counts(total, total)?;
            }
            Ok(p)
        }
        MediaType::Webtoon | MediaType::TvSeries | MediaType::Anime => {
            let mut p = Progress::Episodes { current, total };
            if current > total && total > 0 {
                return Err("current can't be past the total.".to_string());
            }
            if status == Status::Completed && total > 0 {
                p.set_counts(total, total)?;
            }
            Ok(p)
        }
    }
}
