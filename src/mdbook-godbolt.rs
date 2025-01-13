use crate::libgodbolt::GodboltSnippet;
use std::{io, process};

use clap::{Arg, ArgMatches, Command};
use mdbook::BookItem;
use mdbook::book::{Book, Chapter};
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, PreprocessorContext, Preprocessor};
use semver::{Version, VersionReq};

fn make_cli() -> Command {
    Command::new("mdbook-godbolt")
        .about("A preprocessor for mdbook to add runnable code snippets via Godbolt")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by the preprocessor"),
        )
}

fn main() {
    let matches = make_cli().get_matches();

    let preprocessor = GodboltSnippet::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from mdbook version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args
        .get_one::<String>("renderer")
        .expect("Required argument");

    let supported = pre.supports_renderer(renderer);

    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

mod libgodbolt {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    use super::*;

    pub struct GodboltSnippet;

    impl GodboltSnippet {
        pub fn new() -> GodboltSnippet {
            GodboltSnippet
        }
    }

    impl Preprocessor for GodboltSnippet {
        fn name(&self) -> &str {
            "mdbook-godbolt"
        }

        fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
            book.for_each_mut(|item: &mut BookItem| {
                let BookItem::Chapter(ch) = item else {
                    return;
                };

                if ch.is_draft_chapter() {
                    return;
                }

                match godbolt_snippets(ch.content) {
                    Ok(s) => ch.content = s,
                    Err(e) => eprintln!("Failed to process chapter {e:?}"),
                }
            });

            Ok(book)
        }
    }

    fn godbolt_snippets(content: &str) -> Result<String, Error> {
        let opts = Options::empty();
        let mut godbolt_blocks = vec![];

        let events = Parser::new_ext(content, opts);

        for (event, span) in events.into_offset_iter() {
            if let Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(info_string))) = event.clone() {
                let span_content = &content[span.start..span.end];
                const INDENT_MAX
            }
        }

        Ok(String::from(""))
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn godbolt_preprocessor_run() {
            let input_json = r##"[
                {
                    "root": "/path/to/book",
                    "config": {
                        "book": {
                            "authors": ["AUTHOR"],
                            "language": "en",
                            "multilingual": false,
                            "src": "src",
                            "title": "TITLE"
                        },
                        "preprocessor": {
                            "nop": {}
                        }
                    },
                    "renderer": "html",
                    "mdbook_version": "0.4.21"
                },
                {
                    "sections": [
                        {
                            "Chapter": {
                                "name": "Chapter 1",
                                "content": "# Chapter 1\n",
                                "number": [1],
                                "sub_items": [],
                                "path": "chapter_1.md",
                                "source_path": "chapter_1.md",
                                "parent_names": []
                            }
                        }
                    ],
                    "__non_exhaustive": null
                }
            ]"##;
            let input_json = input_json.as_bytes();

            let (ctx, book) = mdbook::preprocess::CmdPreprocessor::parse_input(input_json).unwrap();
            let expected_book = book.clone();
            let result = GodboltSnippet::new().run(&ctx, book);
            assert!(result.is_ok());

            // The nop-preprocessor should not have made any changes to the book content.
            let actual_book = result.unwrap();
            assert_eq!(actual_book, expected_book);
        }
    }
}

