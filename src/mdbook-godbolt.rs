use crate::libgodbolt::GodboltPreprocessor;
use std::{io, process};

use clap::{Arg, ArgMatches, Command};
use mdbook::book::Book;
use mdbook::errors::{Error, Result as MdBookResult};
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use mdbook::BookItem;
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

    let preprocessor = GodboltPreprocessor::new();

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

// TODO: Install of custom book.js to handle godbolt based codeblocks

mod libgodbolt {
    use std::collections::HashMap;

    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

    use super::*;

    pub struct GodboltPreprocessor;

    impl GodboltPreprocessor {
        pub fn new() -> GodboltPreprocessor {
            GodboltPreprocessor
        }
    }

    impl Preprocessor for GodboltPreprocessor {
        fn name(&self) -> &str {
            "mdbook-godbolt"
        }

        fn supports_renderer(&self, renderer: &str) -> bool {
            match renderer {
                "HTML" | "html" => true,
                _ => false,
            }
        }

        fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> MdBookResult<Book> {
            book.for_each_mut(|item: &mut BookItem| {
                let BookItem::Chapter(ch) = item else {
                    return;
                };

                if ch.is_draft_chapter() {
                    return;
                }

                match preprocesses(&ch.content) {
                    Ok(s) => ch.content = s,
                    Err(e) => eprintln!("Failed to process chapter {e:?}"),
                }
            });

            Ok(book)
        }
    }

    struct Godbolt;

    impl Godbolt {
        pub(crate) fn new() -> Self {
            Godbolt
        }

        pub(crate) fn html(self, _id_counter: &mut HashMap<String, usize>) -> String {
            String::from("html")
        }
    }

    fn preprocesses(content: &str) -> MdBookResult<String> {
        let mut id_counter = Default::default();

        // Get markdown parsing events as iterator
        let events = Parser::new_ext(content, Options::empty());

        let mut blocks = vec![];

        // Iterate through events finding codeblocks
        for (event, span) in events.into_offset_iter() {
            if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info_string))) = event.clone()
            {
                let span_content = &content[span.start..span.end];
            }

            let godbolt_block = match parse_codeblock() {
                _ => Ok(()),
            };

            let new_content = godbolt_block.html(&mut id_counter);

            blocks.push((span, new_content));
        }

        // TODO: Add HTML <pre> tag with godbolt class

        // Reconstruct book with added godbolt class
        let content = content.to_string();

        Ok(content)
    }

    fn parse_codeblock() -> MdBookResult<Godbolt> {
        Ok(Godbolt::new())
    }
}
