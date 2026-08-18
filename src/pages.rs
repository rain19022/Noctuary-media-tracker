use crate::models::{MediaItem, MediaType, Progress, Status};
use crate::statistics::Stats;

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

fn url_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            _ if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn layout(active: &str, flash: Option<&str>, body: &str) -> String {
    let nav = |name: &str, href: &str, icon: &str, label: &str, hint: &str| {
        let cls = if active == name { "active" } else { "" };
        format!(
            r#"<a href="{href}" class="{cls}"><span class="nav-icon">{icon}</span><span><strong>{label}</strong><small>{hint}</small></span></a>"#
        )
    };

    let flash_html = flash
        .filter(|s| !s.is_empty())
        .map(|s| format!(r#"<div class="flash">{}</div>"#, esc(s)))
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Noctuary</title>
  <link rel="stylesheet" href="/static/style.css">
</head>
<body>
  <div class="app-shell">
    <aside class="sidebar">
      <a href="/" class="brand">
        <div class="brand-icon">N</div>
        <div class="brand-copy">
          <div class="brand-name">Noctuary</div>
          <div class="brand-sub">a night journal for stories</div>
        </div>
      </a>
      <div class="sidebar-panel">
        <p class="sidebar-kicker">Collection</p>
        <nav class="nav">
          {nav_lib}
          {nav_add}
          {nav_find}
          {nav_numbers}
          {nav_pick}
        </nav>
      </div>
      <div class="sidebar-footer">
        <p>Track books, manga, movies, shows, and anime in one calm place.</p>
      </div>
    </aside>
    <main class="main">
      {flash}
      {body}
    </main>
  </div>
</body>
</html>"#,
        nav_lib = nav("library", "/", "&#9638;", "Library", "Browse the whole stack"),
        nav_add = nav("add", "/add", "+", "Add", "Log a new title"),
        nav_find = nav("find", "/find", "&#9906;", "Discover", "Search and filter"),
        nav_numbers = nav("numbers", "/numbers", "&#9783;", "Stats", "See your pace"),
        nav_pick = nav("pick", "/pick", "?", "Pick", "Choose tonight"),
        flash = flash_html,
        body = body,
    )
}

fn page_hero(
    kicker: &str,
    title: &str,
    subtitle: &str,
    actions: &str,
    stats: &str,
) -> String {
    format!(
        r#"<section class="hero">
  <div class="hero-copy">
    <p class="eyebrow">{kicker}</p>
    <h1>{title}</h1>
    <p class="hero-text">{subtitle}</p>
    <div class="hero-actions">{actions}</div>
  </div>
  <div class="hero-stats">{stats}</div>
</section>"#,
        kicker = kicker,
        title = title,
        subtitle = subtitle,
        actions = actions,
        stats = stats,
    )
}

fn section_header(title: &str, subtitle: &str, action: Option<&str>) -> String {
    format!(
        r#"<div class="section-header">
  <div>
    <h2>{title}</h2>
    <p>{subtitle}</p>
  </div>
  {action}
</div>"#,
        title = title,
        subtitle = subtitle,
        action = action.unwrap_or(""),
    )
}

fn type_badge_class(t: MediaType) -> &'static str {
    match t {
        MediaType::Book => "badge-book",
        MediaType::Manga => "badge-manga",
        MediaType::Webtoon => "badge-webtoon",
        MediaType::Movie => "badge-movie",
        MediaType::TvSeries => "badge-tv",
        MediaType::Anime => "badge-anime",
    }
}

fn poster_class(t: MediaType) -> &'static str {
    match t {
        MediaType::Book => "media-poster-book",
        MediaType::Manga => "media-poster-manga",
        MediaType::Webtoon => "media-poster-webtoon",
        MediaType::Movie => "media-poster-movie",
        MediaType::TvSeries => "media-poster-tv",
        MediaType::Anime => "media-poster-anime",
    }
}

fn status_badge_class(s: Status) -> &'static str {
    match s {
        Status::WantToConsume => "status-want",
        Status::InProgress => "status-now",
        Status::Completed => "status-done",
        Status::OnHold => "status-hold",
        Status::Dropped => "status-dropped",
    }
}

fn progress_unit(t: MediaType) -> &'static str {
    match t {
        MediaType::Book => "page",
        MediaType::Manga | MediaType::Webtoon => "chapter",
        MediaType::TvSeries | MediaType::Anime => "episode",
        MediaType::Movie => "watched",
    }
}

fn progress_unit_plural(t: MediaType) -> &'static str {
    match t {
        MediaType::Book => "pages",
        MediaType::Manga | MediaType::Webtoon => "chapters",
        MediaType::TvSeries | MediaType::Anime => "episodes",
        MediaType::Movie => "watched",
    }
}

fn type_initial(t: MediaType) -> char {
    match t {
        MediaType::Book => 'B',
        MediaType::Manga => 'M',
        MediaType::Webtoon => 'W',
        MediaType::Movie => 'F',
        MediaType::TvSeries => 'T',
        MediaType::Anime => 'A',
    }
}

fn progress_percent(progress: &Progress) -> u32 {
    match progress {
        Progress::Pages { current, total }
        | Progress::Chapters { current, total }
        | Progress::Episodes { current, total } => {
            if *total == 0 {
                0
            } else {
                ((*current as f32 / *total as f32) * 100.0).min(100.0) as u32
            }
        }
        Progress::MovieWatch { watched } => {
            if *watched {
                100
            } else {
                0
            }
        }
    }
}

fn stars_html(rating: Option<u8>) -> String {
    match rating {
        None => r#"<span class="star-none">Not rated</span>"#.to_string(),
        Some(n) => {
            let mut s = String::from(r#"<span class="stars" aria-label="rating">"#);
            for i in 1..=5 {
                if i <= n {
                    s.push_str(r#"<span class="star-on">&#9733;</span>"#);
                } else {
                    s.push_str(r#"<span class="star-off">&#9733;</span>"#);
                }
            }
            s.push_str("</span>");
            s.push_str(&format!(r#"<span class="star-score">{n}/5</span>"#));
            s
        }
    }
}

fn progress_hint(item: &MediaItem) -> String {
    match &item.progress {
        Progress::MovieWatch { watched } => {
            if *watched {
                "Marked watched".to_string()
            } else {
                "Ready to start".to_string()
            }
        }
        Progress::Pages { current, total }
        | Progress::Chapters { current, total }
        | Progress::Episodes { current, total } => {
            let unit = progress_unit_plural(item.media_type);
            if *total == 0 {
                format!("Total {unit} not set yet")
            } else if *current >= *total {
                "Progress complete".to_string()
            } else {
                format!("{} {unit} left", total - current)
            }
        }
    }
}

fn progress_bar_html(progress: &Progress) -> String {
    let pct = progress_percent(progress);
    format!(
        r#"<div class="progress-wrap">
  <div class="progress-bar">
    <div class="progress-fill" style="width:{pct}%"></div>
  </div>
  <div class="progress-label">{label}</div>
</div>"#,
        pct = pct,
        label = esc(&progress.display()),
    )
}

fn genre_tags(genres: &[String]) -> String {
    if genres.is_empty() {
        return r#"<span class="tag tag-muted">No genres yet</span>"#.to_string();
    }
    genres
        .iter()
        .map(|g| format!(r#"<span class="tag">{}</span>"#, esc(g)))
        .collect()
}

fn meta_line(item: &MediaItem) -> String {
    let started = item
        .date_started
        .as_deref()
        .map(esc)
        .unwrap_or_else(|| "No start date".to_string());
    let finished = item
        .date_completed
        .as_deref()
        .map(esc)
        .unwrap_or_else(|| "Not finished".to_string());
    format!(r#"<span>{started}</span><span>{finished}</span>"#)
}

fn media_card(item: &MediaItem) -> String {
    let initial = type_initial(item.media_type);
    format!(
        r#"<article class="media-card">
  <a href="/item/{id}" class="card-link">
    <div class="media-poster {poster}">
      <span class="poster-initial">{initial}</span>
      <div class="poster-badges">
        <span class="badge {type_badge}">{type_label}</span>
        <span class="badge {status_badge}">{status_label}</span>
      </div>
    </div>
    <div class="media-body">
      <div class="media-topline">{meta}</div>
      <h3 class="media-title">{title}</h3>
      <p class="media-subtitle">{hint}</p>
      <div class="tag-row">{genres}</div>
      {progress_bar}
      <div class="media-meta">
        <span class="meta-pill">{status_long}</span>
        <div class="media-rating">{stars}</div>
      </div>
    </div>
  </a>
</article>"#,
        id = item.id,
        poster = poster_class(item.media_type),
        initial = initial,
        type_badge = type_badge_class(item.media_type),
        type_label = item.media_type.label(),
        status_badge = status_badge_class(item.status),
        status_label = item.status.label(),
        meta = meta_line(item),
        title = esc(&item.title),
        hint = esc(&progress_hint(item)),
        genres = genre_tags(&item.genres),
        progress_bar = progress_bar_html(&item.progress),
        status_long = esc(match item.status {
            Status::WantToConsume => "Want to read/watch",
            Status::InProgress => "Currently in progress",
            Status::Completed => "Completed",
            Status::OnHold => "Paused for now",
            Status::Dropped => "Dropped",
        }),
        stars = stars_html(item.rating),
    )
}

fn media_grid(items: &[&MediaItem]) -> String {
    let cards: String = items.iter().map(|i| media_card(i)).collect();
    format!(r#"<div class="media-grid">{cards}</div>"#)
}

fn shelf_section(
    title: &str,
    subtitle: &str,
    items: &[&MediaItem],
    empty_message: &str,
    action: Option<&str>,
) -> String {
    let content = if items.is_empty() {
        format!(r#"<div class="empty empty-inline"><p>{}</p></div>"#, esc(empty_message))
    } else {
        media_grid(items)
    };
    format!(
        r#"<section class="content-section">
  {header}
  {content}
</section>"#,
        header = section_header(title, subtitle, action),
        content = content,
    )
}

fn hero_stat(value: String, label: &str) -> String {
    format!(
        r#"<div class="hero-stat"><strong>{}</strong><span>{}</span></div>"#,
        esc(&value),
        label
    )
}

fn summary_link(href: &str, value: String, label: &str) -> String {
    format!(
        r#"<a href="{href}" class="summary-link"><strong>{}</strong><span>{}</span></a>"#,
        esc(&value),
        label
    )
}

fn fact_card(label: &str, value: &str) -> String {
    format!(
        r#"<div class="fact-card"><span>{}</span><strong>{}</strong></div>"#,
        label,
        esc(value)
    )
}

pub fn library(items: &[MediaItem], flash: Option<&str>) -> String {
    let total = items.len();
    let in_progress: Vec<&MediaItem> = items
        .iter()
        .filter(|item| item.status == Status::InProgress)
        .collect();
    let want: Vec<&MediaItem> = items
        .iter()
        .filter(|item| item.status == Status::WantToConsume)
        .collect();
    let completed: Vec<&MediaItem> = items
        .iter()
        .filter(|item| item.status == Status::Completed)
        .collect();
    let recent: Vec<&MediaItem> = items.iter().rev().take(4).collect();
    let completed_preview: Vec<&MediaItem> = completed.iter().copied().take(4).collect();
    let all_refs: Vec<&MediaItem> = items.iter().collect();

    let hero = page_hero(
        "Noctuary",
        "Your personal media tracker",
        "Keep the whole stack in one place, from quiet reading nights to watch-list marathons.",
        r#"<a href="/add" class="btn btn-primary">Add a title</a><a href="/find" class="btn btn-secondary">Browse filters</a><a href="/pick" class="btn btn-secondary">Pick tonight</a>"#,
        &format!(
            "{}{}{}{}",
            hero_stat(total.to_string(), "titles logged"),
            hero_stat(in_progress.len().to_string(), "in progress"),
            hero_stat(want.len().to_string(), "want list"),
            hero_stat(completed.len().to_string(), "completed")
        ),
    );

    let body = if items.is_empty() {
        format!(
            r#"{hero}
<div class="empty">
  <h2>Nothing here yet</h2>
  <p>Start your noctuary with a first book, movie, manga, or series.</p>
  <a href="/add" class="btn btn-primary">Add your first title</a>
</div>"#
        )
    } else {
        let summary_row = format!(
            r#"<section class="summary-row">
  {}
  {}
  {}
  {}
</section>"#,
            summary_link("/", total.to_string(), "entire library"),
            summary_link("/find?status=InProgress", in_progress.len().to_string(), "currently active"),
            summary_link("/find?status=WantToConsume", want.len().to_string(), "queued next"),
            summary_link("/find?sort=rating", completed.len().to_string(), "worth revisiting"),
        );
        let spotlight = recent.first().map(|item| {
            format!(
                r#"<section class="spotlight-card">
  <div class="spotlight-copy">
    <p class="eyebrow">Recent addition</p>
    <h2>{title}</h2>
    <p>{summary}</p>
    <div class="detail-tags">
      <span class="badge {type_badge}">{type_label}</span>
      <span class="badge {status_badge}">{status_label}</span>
    </div>
    <div class="hero-actions">
      <a href="/item/{id}" class="btn btn-primary">Open profile</a>
      <a href="/pick" class="btn btn-secondary">Pick tonight</a>
    </div>
  </div>
  <div class="spotlight-metrics">
    {rating}
    {progress}
  </div>
</section>"#,
                title = esc(&item.title),
                summary = esc(&progress_hint(item)),
                type_badge = type_badge_class(item.media_type),
                type_label = item.media_type.label(),
                status_badge = status_badge_class(item.status),
                status_label = item.status.label(),
                id = item.id,
                rating = fact_card(
                    "Rating",
                    &item
                        .rating
                        .map(|r| format!("{r}/5"))
                        .unwrap_or_else(|| "Not rated".to_string()),
                ),
                progress = fact_card("Progress", &item.progress.display()),
            )
        });

        format!(
            r#"{hero}
{summary_row}
{spotlight}
{recent_section}
{progress_section}
{want_section}
{completed_section}
{all_section}"#,
            hero = hero,
            summary_row = summary_row,
            spotlight = spotlight.unwrap_or_default(),
            recent_section = shelf_section(
                "Recently added",
                "The latest titles you logged into Noctuary.",
                &recent,
                "Add a title to build this shelf.",
                None,
            ),
            progress_section = shelf_section(
                "Continue where you left off",
                "Your active reads and watches, ready to jump back into.",
                &in_progress,
                "Nothing is currently in progress.",
                Some(r#"<a href="/find" class="text-link">Open discover</a>"#),
            ),
            want_section = shelf_section(
                "Up next",
                "The stack waiting for your next free night.",
                &want,
                "Your want-list is empty right now.",
                Some(r#"<a href="/pick" class="text-link">Random pick</a>"#),
            ),
            completed_section = shelf_section(
                "Finished recently",
                "Titles you already closed out and can revisit anytime.",
                &completed_preview,
                "No completed titles yet.",
                Some(r#"<a href="/numbers" class="text-link">See stats</a>"#),
            ),
            all_section = shelf_section(
                "All titles",
                "Your full collection in one scrollable shelf.",
                &all_refs,
                "No titles logged yet.",
                None,
            ),
        )
    };

    layout("library", flash, &body)
}

fn media_type_options(selected: Option<MediaType>) -> String {
    let opts = [
        (MediaType::Book, "Book"),
        (MediaType::Manga, "Manga"),
        (MediaType::Webtoon, "Webtoon"),
        (MediaType::Movie, "Movie"),
        (MediaType::TvSeries, "TV Series"),
        (MediaType::Anime, "Anime"),
    ];
    opts.iter()
        .map(|(t, label)| {
            let sel = selected == Some(*t);
            format!(
                r#"<option value="{}"{}>{}</option>"#,
                t.key(),
                if sel { " selected" } else { "" },
                label
            )
        })
        .collect()
}

fn status_options(selected: Option<Status>) -> String {
    let opts = [
        (Status::WantToConsume, "Want to watch/read"),
        (Status::InProgress, "In progress"),
        (Status::Completed, "Completed"),
        (Status::OnHold, "On hold"),
        (Status::Dropped, "Dropped"),
    ];
    opts.iter()
        .map(|(s, label)| {
            let sel = selected == Some(*s);
            format!(
                r#"<option value="{}"{}>{}</option>"#,
                s.key(),
                if sel { " selected" } else { "" },
                label
            )
        })
        .collect()
}

pub fn add_form(error: Option<&str>) -> String {
    let err = error
        .map(|e| format!(r#"<div class="error">{}</div>"#, esc(e)))
        .unwrap_or_default();

    let hero = page_hero(
        "New entry",
        "Add something to Noctuary",
        "Capture the title, progress, and small details now so your shelf stays organized later.",
        r#"<a href="/" class="btn btn-secondary">Back to library</a>"#,
        &format!(
            "{}{}",
            hero_stat("6".to_string(), "media types"),
            hero_stat("1".to_string(), "shared form")
        ),
    );

    let body = format!(
        r#"{hero}
{err}
<section class="split-layout">
  <div class="panel panel-form">
    <form method="post" action="/add" class="form">

      <div class="form-section">
        <p class="form-section-label">Identity</p>
        <div class="form-grid">
          <label class="full">Title
            <input type="text" name="title" required placeholder="e.g. Solo Leveling, Dune, Loki...">
          </label>
          <label>Type
            <select name="media_type">{types}</select>
          </label>
          <label>Status
            <select name="status">{statuses}</select>
          </label>
          <label>Genre <span class="hint">comma-separated</span>
            <input type="text" name="genres" placeholder="Action, Fantasy, Slice of life">
          </label>
          <label>Rating <span class="hint">1-5, leave blank if not rated yet</span>
            <input type="number" name="rating" min="1" max="5" placeholder="--">
          </label>
        </div>
      </div>

      <div class="form-section">
        <p class="form-section-label">Progress</p>
        <p class="form-section-hint">Use pages for books, chapters for manga and webtoons, episodes for anime and TV. Leave both at 0 if you have not started yet.</p>
        <div class="form-grid">
          <label>Where you are now <span class="hint">pages / chapters / episodes</span>
            <input type="number" name="current" min="0" value="0">
          </label>
          <label>Total in the whole title <span class="hint">0 if unknown</span>
            <input type="number" name="total" min="0" value="0">
          </label>
          <label class="checkbox-row full">
            <input type="checkbox" name="watched" value="yes">
            <span>Already watched <span class="hint">(movies only)</span></span>
          </label>
        </div>
      </div>

      <div class="form-section">
        <p class="form-section-label">Dates <span class="hint">optional</span></p>
        <div class="form-grid">
          <label>Started <span class="hint">YYYY-MM-DD</span>
            <input type="text" name="date_started" placeholder="2026-01-15">
          </label>
          <label>Finished <span class="hint">YYYY-MM-DD</span>
            <input type="text" name="date_completed" placeholder="2026-03-20">
          </label>
        </div>
      </div>

      <div class="form-section">
        <p class="form-section-label">Notes <span class="hint">optional</span></p>
        <label class="full">
          <textarea name="notes" rows="4" placeholder="Thoughts, favorite scenes, where you paused, or why you picked it up..."></textarea>
        </label>
      </div>

      <div class="form-actions">
        <button type="submit" class="btn btn-primary">Add to Noctuary</button>
        <a href="/" class="btn btn-secondary">Cancel</a>
      </div>
    </form>
  </div>
  <aside class="panel panel-side">
    <h2>Entry guide</h2>
    <ul class="check-list">
      <li><strong>Type matters.</strong> It controls what the progress numbers mean — pages for books, chapters for manga, episodes for series.</li>
      <li><strong>Set the total if you know it.</strong> The progress bar and "X left" hints only work when the total is filled in.</li>
      <li><strong>Leave 0 if you haven't started.</strong> You can update progress anytime from the title's profile page.</li>
      <li><strong>Genres help discover.</strong> A few short tags like "action" or "slice of life" make searches faster later.</li>
      <li><strong>Notes are just for you.</strong> Jot anything — a page you want to reread, a scene you liked, or simply "pick this up again in summer".</li>
    </ul>
  </aside>
</section>"#,
        hero = hero,
        err = err,
        types = media_type_options(None),
        statuses = status_options(Some(Status::WantToConsume)),
    );

    layout("add", None, &body)
}

pub fn find_page(
    items: &[&MediaItem],
    title: &str,
    media_type: Option<MediaType>,
    status: Option<Status>,
    genre: &str,
    sort_by_rating: bool,
) -> String {
    let sort_checked = if sort_by_rating { " checked" } else { "" };
    let active_filters = [!title.trim().is_empty(), media_type.is_some(), status.is_some(), !genre.trim().is_empty(), sort_by_rating]
        .into_iter()
        .filter(|flag| *flag)
        .count();
    let active_filter_tags = [
        (!title.trim().is_empty()).then(|| format!(r#"<span class="filter-chip">Title: {}</span>"#, esc(title))),
        media_type.map(|t| format!(r#"<span class="filter-chip">Type: {}</span>"#, esc(t.label()))),
        status.map(|s| format!(r#"<span class="filter-chip">Status: {}</span>"#, esc(s.label()))),
        (!genre.trim().is_empty()).then(|| format!(r#"<span class="filter-chip">Genre: {}</span>"#, esc(genre))),
        sort_by_rating.then(|| r#"<span class="filter-chip">Sorted by rating</span>"#.to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<String>();

    let hero = page_hero(
        "Discover",
        "Search your collection with focus",
        "Find one title fast, or narrow the shelf by type, status, genre, and rating order.",
        r#"<a href="/find" class="btn btn-secondary">Clear filters</a>"#,
        &format!(
            "{}{}",
            hero_stat(items.len().to_string(), "results"),
            hero_stat(active_filters.to_string(), "active filters")
        ),
    );

    let results = if items.is_empty() {
        r#"<div class="empty">
  <h2>No matches</h2>
  <p>Try a broader title search, or remove one filter and search again.</p>
</div>"#
            .to_string()
    } else {
        format!(
            r#"<div class="result-summary">
  <p>{count} result(s)</p>
</div>
{grid}"#,
            count = items.len(),
            grid = media_grid(items),
        )
    };

    let body = format!(
        r#"{hero}
{active_filters}
<section class="panel panel-search">
  <form method="get" action="/find" class="filter-bar">
    <label>Title
      <input type="text" name="title" value="{title}" placeholder="Search...">
    </label>
    <label>Type
      <select name="media_type">
        <option value="">All types</option>
        {types}
      </select>
    </label>
    <label>Status
      <select name="status">
        <option value="">All statuses</option>
        {statuses}
      </select>
    </label>
    <label>Genre
      <input type="text" name="genre" value="{genre}" placeholder="e.g. Action">
    </label>
    <label class="checkbox-row">
      <input type="checkbox" name="sort" value="rating"{sort_checked}> Sort by rating
    </label>
    <button type="submit" class="btn btn-primary">Search</button>
  </form>
</section>
<section class="content-section">
  {results}
</section>"#,
        hero = hero,
        active_filters = if active_filter_tags.is_empty() {
            String::new()
        } else {
            format!(r#"<div class="filter-chip-row">{active_filter_tags}</div>"#)
        },
        title = esc(title),
        types = media_type_options(media_type),
        statuses = status_options(status),
        genre = esc(genre),
        sort_checked = sort_checked,
        results = results,
    );

    layout("find", None, &body)
}

pub fn item_detail(item: &MediaItem, error: Option<&str>) -> String {
    let err = error
        .map(|e| format!(r#"<div class="error">{}</div>"#, esc(e)))
        .unwrap_or_default();
    let initial = type_initial(item.media_type);

    let progress_form = match item.progress {
        Progress::MovieWatch { watched } => format!(
            r#"<form method="post" action="/item/{id}/progress" class="form form-row">
  <label class="checkbox-row">
    <input type="checkbox" name="watched" value="yes"{checked}> Watched
  </label>
  <button type="submit" class="btn btn-primary">Save</button>
</form>"#,
            id = item.id,
            checked = if watched { " checked" } else { "" },
        ),
        _ => {
            let (current, total) = item.progress.counts().unwrap_or((0, 0));
            let unit = progress_unit(item.media_type);
            let units = progress_unit_plural(item.media_type);
            let pct = if total > 0 {
                (current as f32 / total as f32 * 100.0).min(100.0) as u32
            } else {
                0
            };
            let remaining = if total > 0 && current < total {
                format!("{} {} remaining", total - current, units)
            } else if total > 0 && current >= total {
                "All done".to_string()
            } else {
                format!("Set total {units} to track progress")
            };
            format!(
                r#"<div class="progress-inline">
  <div class="progress-bar"><div class="progress-fill" style="width:{pct}%"></div></div>
  <p class="progress-inline-hint">{remaining}</p>
</div>
<form method="post" action="/item/{id}/progress" class="form">
  <div class="form-grid">
    <label>Current {unit}
      <input type="number" name="current" min="0" value="{current}">
    </label>
    <label>Total {units} <span class="hint">0 if unknown</span>
      <input type="number" name="total" min="0" value="{total}">
    </label>
  </div>
  <div class="form-actions">
    <button type="submit" class="btn btn-primary">Save progress</button>
  </div>
</form>"#,
                pct = pct,
                remaining = esc(&remaining),
                id = item.id,
                unit = unit,
                units = units,
                current = current,
                total = total,
            )
        }
    };

    let notes_block = if item.notes.is_empty() {
        r#"<div class="detail-notes detail-notes-empty">No notes yet.</div>"#.to_string()
    } else {
        format!(r#"<div class="detail-notes">{}</div>"#, esc(&item.notes))
    };

    let genre_value = if item.genres.is_empty() {
        "None".to_string()
    } else {
        item.genre_line()
    };

    let facts = format!(
        "{}{}{}{}{}{}",
        fact_card("Type", item.media_type.label()),
        fact_card("Status", item.status.label()),
        fact_card(
            "Started",
            item.date_started.as_deref().unwrap_or("Not set")
        ),
        fact_card(
            "Finished",
            item.date_completed.as_deref().unwrap_or("Not set")
        ),
        fact_card("Progress", &item.progress.display()),
        fact_card("Genres", &genre_value)
    );

    let hero = format!(
        r#"<section class="detail-hero">
  <div class="detail-poster media-poster {poster}">
    <span class="poster-initial poster-initial-large">{initial}</span>
  </div>
  <div class="detail-info">
    <p class="eyebrow">Title profile</p>
    <h1>{title}</h1>
    <div class="detail-tags">
      <span class="badge {type_badge}">{type_label}</span>
      <span class="badge {status_badge}">{status_label}</span>
      {genre_tags}
    </div>
    <div class="detail-rating">{stars}</div>
    <p class="detail-summary">{summary}</p>
    <div class="hero-actions">
      <a href="/" class="btn btn-secondary">Back to library</a>
      <a href="/find?title={encoded_title}" class="btn btn-secondary">Find similar</a>
    </div>
    {progress_bar}
    <div class="facts-grid">{facts}</div>
  </div>
</section>"#,
        poster = poster_class(item.media_type),
        initial = initial,
        title = esc(&item.title),
        type_badge = type_badge_class(item.media_type),
        type_label = item.media_type.label(),
        status_badge = status_badge_class(item.status),
        status_label = item.status.label(),
        genre_tags = genre_tags(&item.genres),
        stars = stars_html(item.rating),
        summary = esc(&progress_hint(item)),
        encoded_title = url_component(&item.title),
        progress_bar = progress_bar_html(&item.progress),
        facts = facts,
    );

    let body = format!(
        r#"{err}
{hero}
<section class="detail-layout">
  <div class="detail-main">
    <div class="action-card">
      <h3>Notes</h3>
      {notes_block}
    </div>
  </div>
  <div class="detail-side">
    <div class="action-card">
      <h3>Progress</h3>
      {progress_form}
    </div>
    <div class="action-card">
      <h3>Status</h3>
      <form method="post" action="/item/{id}/status" class="form form-row">
        <label class="field-grow"><select name="status">{statuses}</select></label>
        <button type="submit" class="btn btn-primary">Save</button>
      </form>
    </div>
    <div class="action-card">
      <h3>Rating</h3>
      <form method="post" action="/item/{id}/rating" class="form form-row">
        <label class="field-grow"><input type="number" name="rating" min="1" max="5" value="{rating_val}" placeholder="1-5"></label>
        <button type="submit" class="btn btn-primary">Save</button>
      </form>
    </div>
    <div class="danger-zone">
      <form method="post" action="/item/{id}/delete">
        <button type="submit" class="btn btn-danger btn-block">Remove from Noctuary</button>
      </form>
    </div>
  </div>
</section>"#,
        err = err,
        hero = hero,
        notes_block = notes_block,
        progress_form = progress_form,
        id = item.id,
        statuses = status_options(Some(item.status)),
        rating_val = item.rating.map(|r| r.to_string()).unwrap_or_default(),
    );

    layout("library", None, &body)
}

fn stat_card(value: &str, label: &str, note: &str, highlight: bool) -> String {
    let cls = if highlight {
        "stat-card highlight"
    } else {
        "stat-card"
    };
    format!(
        r#"<div class="{cls}"><div class="value">{value}</div><div class="label">{label}</div><p class="stat-note">{note}</p></div>"#
    )
}

pub fn numbers(stats: &Stats) -> String {
    let avg = match stats.average_rating {
        Some(a) => format!("{a:.1}/5"),
        None => "--".to_string(),
    };
    let completion_rate = if stats.total == 0 {
        0
    } else {
        (stats.completed * 100) / stats.total
    };

    let hero = page_hero(
        "Stats",
        "Your reading and watching at a glance",
        "A quick view of pace, completion, and which kinds of stories fill your shelf most.",
        r#"<a href="/" class="btn btn-secondary">Back to library</a>"#,
        &format!(
            "{}{}",
            hero_stat(stats.total.to_string(), "titles tracked"),
            hero_stat(format!("{completion_rate}%"), "completion rate")
        ),
    );

    let body = format!(
        r#"{hero}
<section class="stats-hero-band">
  <div class="stats-hero-copy">
    <h2>Completion snapshot</h2>
    <p>{completed} of {total} titles are completed, with {in_progress} still active right now.</p>
  </div>
  <div class="stats-progress">
    <div class="progress-bar progress-bar-large">
      <div class="progress-fill" style="width:{completion_rate}%"></div>
    </div>
    <span>{completion_rate}% complete</span>
  </div>
</section>
<div class="stats-grid">
  {total_card}
  {done}
  {now}
  {avg_card}
</div>
<section class="content-section">
  {type_header}
  <div class="stats-grid stats-grid-compact">
    {book}
    {manga}
    {webtoon}
    {movie}
    {tv}
    {anime}
  </div>
</section>"#,
        hero = hero,
        completed = stats.completed,
        total = stats.total,
        in_progress = stats.in_progress,
        completion_rate = completion_rate,
        total_card = stat_card(&stats.total.to_string(), "Total titles", "Everything logged so far", false),
        done = stat_card(&stats.completed.to_string(), "Completed", "Finished across all types", true),
        now = stat_card(&stats.in_progress.to_string(), "In progress", "Active right now", false),
        avg_card = stat_card(&avg, "Average rating", "Across rated titles", false),
        type_header = section_header("Completed by type", "Which shelves you finish the most.", None),
        book = stat_card(&stats.completed_book.to_string(), "Books", "Finished books", false),
        manga = stat_card(&stats.completed_manga.to_string(), "Manga", "Finished manga", false),
        webtoon = stat_card(&stats.completed_webtoon.to_string(), "Webtoons", "Finished webtoons", false),
        movie = stat_card(&stats.completed_movie.to_string(), "Movies", "Finished movies", false),
        tv = stat_card(&stats.completed_tv.to_string(), "TV", "Finished series", false),
        anime = stat_card(&stats.completed_anime.to_string(), "Anime", "Finished anime", false),
    );

    layout("numbers", None, &body)
}

pub fn pick(item: Option<&MediaItem>) -> String {
    let hero = page_hero(
        "Pick",
        "Let Noctuary choose for tonight",
        "A calmer way to break indecision when the want-list gets too full.",
        r#"<a href="/find" class="btn btn-secondary">Open discover</a>"#,
        &hero_stat("1".to_string(), "random title"),
    );

    let body = match item {
        None => format!(
            r#"{hero}
<div class="empty">
  <h2>Want-list is empty</h2>
  <p>Add titles with status \"want\" first, then come back for a quick pick.</p>
  <a href="/add" class="btn btn-primary">Add something</a>
</div>"#
        ),
        Some(item) => {
            let initial = type_initial(item.media_type);
            format!(
                r#"{hero}
<article class="pick-card">
  <div class="media-poster {poster}">
    <span class="poster-initial">{initial}</span>
  </div>
  <div class="media-body">
    <p class="pick-label">Tonight's pick</p>
    <h2 class="pick-title">{title}</h2>
    <p class="pick-sub">{summary}</p>
    <div class="detail-tags pick-tags">
      <span class="badge {type_badge}">{type_label}</span>
      <span class="badge {status_badge}">{status_label}</span>
      {genres}
    </div>
    <div class="pick-actions">
      <a href="/item/{id}" class="btn btn-primary">View details</a>
      <a href="/pick" class="btn btn-secondary">Pick again</a>
    </div>
  </div>
</article>"#,
                hero = hero,
                poster = poster_class(item.media_type),
                initial = initial,
                title = esc(&item.title),
                summary = esc(&progress_hint(item)),
                type_badge = type_badge_class(item.media_type),
                type_label = item.media_type.label(),
                status_badge = status_badge_class(item.status),
                status_label = item.status.label(),
                genres = genre_tags(&item.genres),
                id = item.id,
            )
        }
    };

    layout("pick", None, &body)
}

pub fn not_found() -> String {
    let body = r#"<div class="empty">
  <h2>Not found</h2>
  <p>That title profile does not exist in your noctuary.</p>
  <a href="/" class="btn btn-secondary">Back to library</a>
</div>"#;
    layout("library", None, body)
}
