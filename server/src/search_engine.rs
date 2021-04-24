use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{schema::*, Index, IndexReader};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NoteData {
    pub file: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

pub struct SearchEngine {
    index: Index,
    file_field: Field,
    title_field: Field,
    tags_field: Field,
    content_field: Field,
    reader: IndexReader,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RetrievedNote {
    pub file: String,
    pub title: String,
    pub tags: Vec<String>,
    pub score: f32,
}

impl SearchEngine {
    /// Creates a SearchEngine instance
    pub fn build(note_data_vec: Vec<NoteData>, index_path: &str) -> tantivy::Result<Self> {
        // Delete the contents of the index directory if it exists
        if Path::new(index_path).exists() {
            fs::remove_dir_all(index_path)?;
        }
        fs::create_dir(index_path)?;

        // Build the tantivy schema
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("file", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("tags", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT);
        let schema = schema_builder.build();

        // Create the tantivy index
        let index = Index::create_in_dir(&index_path, schema.clone())?;
        let mut index_writer = index.writer(50_000_000)?;

        // Define a field for each piece of stored searchable data
        let file_field = schema.get_field("file").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let tags_field = schema.get_field("tags").unwrap();
        let content_field = schema.get_field("content").unwrap();

        // Add a new tantivy document for each provided note
        for summary in note_data_vec.iter() {
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

        Ok(SearchEngine {
            index,
            file_field,
            title_field,
            tags_field,
            content_field,
            reader,
        })
    }

    pub fn search(&self, search_term: &str, limit: usize) -> Option<Vec<RetrievedNote>> {
        let searcher = self.reader.searcher();
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.file_field,
                self.title_field,
                self.tags_field,
                self.content_field,
            ],
        );
        query_parser.set_conjunction_by_default();

        // Build and run the query
        let query = query_parser.parse_query(search_term).ok()?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit)).ok()?;

        // Parse the query response into a suitable return type
        let mut responses: Vec<RetrievedNote> = Vec::new();
        for (doc_score, doc_address) in top_docs {
            // If we can find the document, add it to the responses vector
            if let Ok(retrieved_doc) = searcher.doc(doc_address) {
                responses.push(RetrievedNote {
                    file: field_to_string(&retrieved_doc, self.file_field),
                    title: field_to_string(&retrieved_doc, self.title_field),
                    tags: field_to_string_vec(&retrieved_doc, self.tags_field),
                    score: doc_score,
                });
            }
        }

        Some(responses)
    }
}

fn field_to_string(doc: &Document, field: Field) -> String {
    if let Some(x) = doc.get_first(field) {
        String::from(x.text().unwrap_or_default())
    } else {
        String::from("")
    }
}

fn field_to_string_vec(doc: &Document, field: Field) -> Vec<String> {
    doc.get_all(field)
        .into_iter()
        .map(|field| String::from(field.text().unwrap_or_default()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn dummy_note_data_vec() -> Vec<NoteData> {
        vec![
            NoteData {
                file: String::from("file1.md"),
                title: String::from("file 1"),
                tags: vec![String::from("first_tag"), String::from("second_tag")],
                content: String::from(
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
                    tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, \
                    quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo \
                    consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse \
                    cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non \
                    proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
                ),
            },
            NoteData {
                file: String::from("file2.md"),
                title: String::from("file 2"),
                tags: vec![String::from("first_tag")],
                content: String::from(
                    "Pellentesque sed ex vel elit vestibulum euismod. In non ipsum in turpis \
                    sollicitudin aliquet sit amet vitae ante. Vestibulum nec mauris sit amet \
                    quam varius eleifend. Donec porttitor risus ante, et pulvinar lorem mollis \
                    in. Integer aliquet finibus leo sollicitudin finibus. Etiam dignissim arcu \
                    tempor, commodo magna molestie, malesuada leo.",
                ),
            },
        ]
    }

    #[test]
    fn it_builds() {
        let index_dir = tempdir().unwrap();
        let engine = SearchEngine::build(dummy_note_data_vec(), index_dir.path().to_str().unwrap());
        assert!(engine.is_ok());
    }

    #[test]
    fn it_searches_filenames() {
        let index_dir = tempdir().unwrap();
        let engine =
            SearchEngine::build(dummy_note_data_vec(), index_dir.path().to_str().unwrap()).unwrap();

        let search_result = engine.search("file:file1.md", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file1.md");

        let search_result = engine.search("file:file2.md", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file2.md");
    }

    #[test]
    fn it_searches_titles() {
        let index_dir = tempdir().unwrap();
        let engine =
            SearchEngine::build(dummy_note_data_vec(), index_dir.path().to_str().unwrap()).unwrap();

        let search_result = engine.search("title:1", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file1.md");

        let search_result = engine.search("title:2", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file2.md");
    }

    #[test]
    fn it_searches_tags() {
        let index_dir = tempdir().unwrap();
        let engine =
            SearchEngine::build(dummy_note_data_vec(), index_dir.path().to_str().unwrap()).unwrap();

        let search_result = engine.search("tags:first_tag", 10).unwrap();
        assert_eq!(search_result.len(), 2);

        let search_result = engine.search("tags:second_tag", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file1.md");
    }

    #[test]
    fn it_searches_all() {
        let index_dir = tempdir().unwrap();
        let engine =
            SearchEngine::build(dummy_note_data_vec(), index_dir.path().to_str().unwrap()).unwrap();

        let search_result = engine.search("adipiscing", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file1.md");

        let search_result = engine.search("vestibulum", 10).unwrap();
        assert_eq!(search_result.len(), 1);
        assert_eq!(search_result.get(0).unwrap().file, "file2.md");
    }
}
