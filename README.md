# Noctuary

personal media tracker. books, manga, webtoons, movies, tv, anime.

Noctuary is a night journal for stories. frontend + backend in one Rust app. the backend serves HTML pages; data lives in `data/library.json`. no extra crates.

## run

stop any old terminal session first (`Ctrl+C`), then:

```powershell
cd "d:\rust PT\personal_media_tracker"
.\run.ps1 run
```

open http://127.0.0.1:3000

on this Windows setup, use `.\run.ps1` if plain `cargo` complains about `link.exe`.

## pages

- `/` — library
- `/add` — add something to the noctuary
- `/find` — filter by title, type, status, genre; sort by rating
- `/item/:id` — one item, update progress / status / rating, remove
- `/numbers` — counts and average rating
- `/pick` — random pick from the want-list

## layout

```
src/main.rs         server startup
src/server.rs       small HTTP backend (std only)
src/routes.rs       request handlers
src/pages.rs        html (frontend)
src/models.rs       MediaType, Status, Progress, MediaItem
src/media.rs        search, filter, sort
src/storage.rs      json read/write
src/statistics.rs   counts
static/style.css
data/library.json
```

## tests

```powershell
.\run.ps1 test
```
