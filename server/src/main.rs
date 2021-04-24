#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod configuration;
mod search_engine;

use configuration::Configuration;
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
fn notes(configuration_state: State<Configuration>, note: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new(&configuration_state.html_dir_path).join(note)).ok()
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

fn main() {
    // Parse command line arguments configuration
    let configuration = Configuration::get();

    // Read & parse summary JSON file
    let summary_json = fs::read_to_string(&configuration.summary_file_path)
        .expect("ERROR: Unable to read summary JSON file");
    let note_data_vec: Vec<NoteData> = serde_json::from_str(summary_json.as_str())
        .expect("ERROR: Unable to parse summary JSON file");

    // Build search engine
    let engine = SearchEngine::build(note_data_vec, "./.index")
        .expect("ERROR: could not build full-text search index");

    // Build and launch rocket server
    rocket::ignite()
        .attach(Template::fairing())
        .manage(configuration)
        .manage(engine)
        .mount("/", routes![index, notes, search, favicon])
        .mount(
            "/static",
            StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
        .launch();
}
