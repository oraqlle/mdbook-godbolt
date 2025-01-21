use crate::libgodbolt::GodboltSnippet;
use std::{io, process};
use std::ops::Range;

use clap::{Arg, ArgMatches, Command};
use mdbook::BookItem;
use mdbook::book::{Book};
use mdbook::errors::{Error, Result as MdBookResult};
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
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

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

        fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> MdBookResult<Book> {
            book.for_each_mut(|item: &mut BookItem| {
                let BookItem::Chapter(ch) = item else {
                    return;
                };

                if ch.is_draft_chapter() {
                    return;
                }

                match godbolt_snippets(&ch.content) {
                    Ok(s) => ch.content = s,
                    Err(e) => eprintln!("Failed to process chapter {e:?}"),
                }
            });

            Ok(book)
        }
    }

    fn godbolt_snippets(content: &str) -> MdBookResult<String> {

        let mut buf = String::with_capacity(content.len());

        let events = Parser::new(&content).filter(|e| match e {
            Event::Start(Tag::Emphasis) | Event::Start(Tag::Strong) => {
                false
            }
            Event::End(TagEnd::Emphasis) | Event::End(TagEnd::Strong) => false,
            _ => true,
        });

        Ok(pulldown_cmark_to_cmark::cmark(events, &mut buf).map(|_| buf)?)
    }
}

