#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod search_engine;

use rocket::response::NamedFile;
use rocket::State;
use rocket_contrib::json::Json;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use search_engine::{NoteData, RetrievedNote, SearchEngine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::{env, path::PathBuf};

#[get("/")]
fn index() -> Template {
    Template::render("index", ())
}

#[get("/favicon.ico")]
fn favicon() -> Option<NamedFile> {
    NamedFile::open("static/favicon.ico").ok()
}

#[post("/notes/<note..>")]
fn notes(cli_args_state: State<CliArguments>, note: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new(&cli_args_state.notes_dir_path).join(note)).ok()
}

#[derive(Serialize, Deserialize)]
struct SearchRequest {
    search_term: String,
}

#[derive(Default, Serialize, Deserialize)]
struct SearchResponse {
    responses: Vec<RetrievedNote>,
}

#[post("/search", format = "json", data = "<request>")]
fn search(engine: State<SearchEngine>, request: Json<SearchRequest>) -> Json<SearchResponse> {
    // If the search engine returns a result, send it, otherwise send an empty response object
    match engine.search(request.search_term.as_str(), 100) {
        Some(x) => Json(SearchResponse { responses: x }),
        None => Json(SearchResponse::default()),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CliArguments {
    summary_file_path: String,
    notes_dir_path: String,
}

impl CliArguments {
    fn get() -> Self {
        let args: Vec<String> = env::args().collect();

        if args.len() != 3 {
            panic!(
                "ERROR: incorrect arguments - usage:\n\
                $ notes_serve [/path/to/summary.json] [/path/to/notes/dir]"
            )
        }

        // Basic sanity checking
        let summary_file_path = Path::new(&args[1]);
        if !summary_file_path.exists() {
            panic!("ERROR: summary JSON file path does not exist")
        }
        if !summary_file_path.is_file() {
            panic!("ERROR: summary JSON file path does not point to a file")
        }

        let notes_dir_path = Path::new(&args[2]);
        if !notes_dir_path.exists() {
            panic!("ERROR: notes directory path does not exist")
        }
        if !notes_dir_path.is_dir() {
            panic!("ERROR: notes directory path does not point to a directory")
        }

        Self {
            summary_file_path: String::from(&args[1]),
            notes_dir_path: String::from(&args[2]),
        }
    }
}

fn main() {
    // Parse command line arguments
    let cli_args = CliArguments::get();

    // Read & parse summary JSON file
    let summary_json = fs::read_to_string(&cli_args.summary_file_path)
        .expect("ERROR: Unable to read summary JSON file");
    let note_data_vec: Vec<NoteData> = serde_json::from_str(summary_json.as_str())
        .expect("ERROR: Unable to parse summary JSON file");

    // Build search engine
    let engine = SearchEngine::build(note_data_vec, "./index")
        .expect("ERROR: could not build full-text search index");

    // Build and launch rocket server
    rocket::ignite()
        .attach(Template::fairing())
        .manage(cli_args)
        .manage(engine)
        .mount("/", routes![index, notes, search, favicon])
        .mount(
            "/static",
            StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
        .launch();
}
