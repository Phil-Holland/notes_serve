use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Configuration {
    pub summary_file_path: String,
    pub html_dir_path: String,
}

impl Configuration {
    pub fn get() -> Self {
        let path_exists = |path: &str| {
            if fs::metadata(path).is_ok() {
                Ok(())
            } else {
                Err(String::from("Path doesn't exist"))
            }
        };

        let matches = App::new("Notes Serve")
            .version("1.0")
            .author("Philip H. <pwaholland@gmail.com>")
            .about(
                "Creates and launches a rocket server, serving markdown notes \
                rendered using the \"notes_serve/renderer\" sibling project",
            )
            .arg(
                Arg::new("summary_file")
                    .short('s')
                    .long("summary_file")
                    .value_name("FILE")
                    .validator(path_exists)
                    .required(true)
                    .about(
                        "Specifies the location of the summary.json file output \
                        by the \"notes_serve/renderer\" sibling project",
                    )
                    .takes_value(true),
            )
            .arg(
                Arg::new("html_dir")
                    .short('d')
                    .long("html_dir")
                    .value_name("DIR")
                    .validator(path_exists)
                    .required(true)
                    .about(
                        "Specifies the location of the rendered HTML output \
                        by the \"notes_serve/renderer\" sibling project",
                    )
                    .takes_value(true),
            )
            .get_matches();

        Self {
            summary_file_path: String::from(
                matches
                    .value_of("summary_file")
                    .expect("Failed to parse argument: summary_file"),
            ),
            html_dir_path: String::from(
                matches
                    .value_of("html_dir")
                    .expect("Failed to parse argument: html_dir"),
            ),
        }
    }
}
