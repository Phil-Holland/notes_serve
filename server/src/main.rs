#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use rocket::response::NamedFile;
use rocket::State;
use rocket_contrib::json::Json;
use rocket_contrib::serve::StaticFiles;
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::{env, path::PathBuf};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{schema::*, Index, IndexReader};

#[derive(Debug, Serialize, Deserialize, Default)]
struct NoteSummary {
    file: String,
    title: String,
    tags: Vec<String>,
    content: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct NoteResponse {
    file: String,
    title: String,
    tags: Vec<String>,
    score: f32,
}

#[derive(Default, Serialize, Deserialize)]
struct SearchResponse {
    responses: Vec<NoteResponse>,
}

struct Indexer {
    index: Index,
    file_field: Field,
    title_field: Field,
    tags_field: Field,
    content_field: Field,
    reader: IndexReader,
}

fn extract_string_from_field(doc: &Document, field: Field) -> String {
    let field = doc.get_first(field);

    let val = match field {
        Some(x) => x.text(),
        None => None,
    };

    match val {
        Some(x) => String::from(x),
        None => String::from(""),
    }
}

fn extract_string_vec_from_field(doc: &Document, field: Field) -> Vec<String> {
    let fields = doc.get_all(field);
    let mut output: Vec<String> = Vec::new();

    for field in fields {
        output.push(match field.text() {
            Some(x) => String::from(x),
            None => String::from(""),
        });
    }

    output
}

#[get("/")]
fn index() -> Template {
    Template::render("index", ())
}

#[post("/notes/<note..>")]
fn notes(cli_args_state: State<CliArguments>, note: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new(&cli_args_state.notes_dir_path).join(note)).ok()
}

#[post("/search", data = "<term>")]
fn search(indexer: State<Indexer>, term: String) -> Json<SearchResponse> {
    let searcher = indexer.reader.searcher();
    let query_parser = QueryParser::for_index(
        &indexer.index,
        vec![
            indexer.file_field,
            indexer.title_field,
            indexer.tags_field,
            indexer.content_field,
        ],
    );

    let query = match query_parser.parse_query(term.as_str()) {
        Ok(query) => query,
        Err(_) => return Json(SearchResponse::default()),
    };

    let top_docs = match searcher.search(&query, &TopDocs::with_limit(100)) {
        Ok(top_docs) => top_docs,
        Err(_) => return Json(SearchResponse::default()),
    };

    let mut responses: Vec<NoteResponse> = Vec::new();
    for (doc_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address).expect("Could not find document");

        let file_string = extract_string_from_field(&retrieved_doc, indexer.file_field);
        let title_string = extract_string_from_field(&retrieved_doc, indexer.title_field);
        let tags_string_vec = extract_string_vec_from_field(&retrieved_doc, indexer.tags_field);

        responses.push(NoteResponse {
            file: file_string,
            title: title_string,
            tags: tags_string_vec,
            score: doc_score,
        });
    }
    Json(SearchResponse { responses })
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

fn build_indexer(summaries: Vec<NoteSummary>) -> tantivy::Result<Indexer> {
    let index_path = "./index";

    // Delete the contents of the index directory if it exists
    if Path::new(index_path).exists() {
        fs::remove_dir_all(index_path)?;
    }
    fs::create_dir(index_path)?;

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("file", TEXT | STORED);
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("tags", TEXT | STORED);
    schema_builder.add_text_field("content", TEXT);
    let schema = schema_builder.build();

    let index = Index::create_in_dir(&index_path, schema.clone())?;
    let mut index_writer = index.writer(50_000_000)?;

    let file_field = schema.get_field("file").unwrap();
    let title_field = schema.get_field("title").unwrap();
    let tags_field = schema.get_field("tags").unwrap();
    let content_field = schema.get_field("content").unwrap();

    for summary in summaries.iter() {
        let mut document = Document::default();
        document.add_text(file_field, summary.file.clone());
        document.add_text(title_field, summary.title.clone());
        document.add_text(content_field, summary.content.clone());
        for summary_tag in summary.tags.iter() {
            document.add_text(tags_field, summary_tag.clone());
        }
        index_writer.add_document(document);
    }

    index_writer.commit()?;

    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::OnCommit)
        .try_into()?;

    Ok(Indexer {
        index,
        file_field,
        title_field,
        tags_field,
        content_field,
        reader,
    })
}

fn main() {
    // Parse command line arguments
    let cli_args = CliArguments::get();

    // Read & parse summary JSON file
    let summary_json = fs::read_to_string(&cli_args.summary_file_path)
        .expect("ERROR: Unable to read summary JSON file");
    let summaries: Vec<NoteSummary> = serde_json::from_str(summary_json.as_str())
        .expect("ERROR: Unable to parse summary JSON file");

    // Build indexer object
    let indexer = build_indexer(summaries).expect("ERROR: could not build full-text search index");

    rocket::ignite()
        .attach(Template::fairing())
        .manage(cli_args)
        .manage(indexer)
        .mount("/", routes![index, notes, search])
        .mount(
            "/static",
            StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
        .launch();
}
