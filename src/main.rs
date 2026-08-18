mod media;
mod models;
mod pages;
mod routes;
mod server;
mod statistics;
mod storage;

use std::io::ErrorKind;
use std::sync::{Arc, Mutex};

use routes::AppState;

fn main() {
    let library = storage::load();
    let state = Arc::new(AppState {
        library: Mutex::new(library),
    });

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("127.0.0.1:{port}");
    println!("Noctuary at http://{addr}");

    let state_for_server = Arc::clone(&state);
    if let Err(err) = server::run(&addr, move |request| routes::handle(&state_for_server, request))
    {
        if err.kind() == ErrorKind::AddrInUse {
            eprintln!("port {port} is already in use.");
            eprintln!("stop the other Noctuary server, or run with: $env:PORT={}", port.parse::<u16>().unwrap_or(3000) + 1);
            eprintln!(r#"then: .\run.ps1 run"#);
            std::process::exit(1);
        }
        eprintln!("server failed: {err}");
        std::process::exit(1);
    }
}
