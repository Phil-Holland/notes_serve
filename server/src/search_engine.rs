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
