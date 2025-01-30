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

    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};

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
            let mut result = None;

            // Iterate through each chapter of the book
            book.for_each_mut(|item: &mut BookItem| {

                if let Some(Err(_)) = result {
                   return; 
                }

                let BookItem::Chapter(ch) = item else {
                    return;
                };

                if ch.is_draft_chapter() {
                    return;
                }

                result = Some(preprocesses(&ch.content).map(|md| ch.content = md));
            });

            // If an error occurred return book as is
            result.unwrap_or(Ok(())).map(|_| book)
        }
    }

    struct GodboltMeta {
        lang: String
    }

    impl GodboltMeta {
        fn new(info_string: &str) -> Option<GodboltMeta> {
            info_string
                .find(',')
                .map(|comma_idx| {

                    if comma_idx == 0 {
                        return None;
                    }

                    let godbolt_info = &info_string[(comma_idx + 1)..];

                    if !godbolt_info.starts_with("godbolt") {
                        return None;
                    }

                    let lang = &info_string[..comma_idx];

                    Some(GodboltMeta { lang: lang.to_string() })
                }).flatten()
        }
    }

    struct Godbolt {
        info: GodboltMeta,
        codeblock: String
    }

    impl Godbolt {
        pub(crate) fn new(info: GodboltMeta, codeblock: String) -> Self {
            Self {
                info,
                codeblock
            }
        }

        pub(crate) fn add_godbolt_pre(self) -> String {
            let html = mdbook::utils::render_markdown(&self.codeblock, false);

            let code_start_idx = html.find("<code").unwrap();
            let code_end_idx = html.find("</code>").unwrap() + 7;
            let code_block = &html[code_start_idx..code_end_idx];

            format!("<pre><pre class=\"godbolt\">{}</pre></pre>", code_block)
        }
    }

    fn preprocesses(content: &str) -> MdBookResult<String> {

        // Get markdown parsing events as iterator
        let events = Parser::new_ext(content, Options::empty());

        let mut godbolt_blocks = vec![];

        // Iterate through events finding codeblocks
        for (event, span) in events.into_offset_iter() {
            if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info_string))) = event.clone()
            {
                let code_content = &content[span.start..span.end];

                let godbolt = match extract_godbolt_info(
                    info_string.as_ref(),
                    code_content) {
                    Some(gd) => gd,
                    None => continue,
                };

                // Adds HTML data around codeblock content
                // TODO: Add HTML <pre> tag with godbolt class
                let godbolt_content = godbolt.add_godbolt_pre();

                // Locally store preprocessed blocks
                godbolt_blocks.push((span, godbolt_content));
            }
        }

        // Reconstruct book with added godbolt class
        let mut new_content = content.to_string();

        // Patch in modified codeblocks into existing book content.
        // This puts the parsed codeblock with meta info back in
        // the correct spot in the book
        for (span, block) in godbolt_blocks.iter().rev() {
            let pre_content = &new_content[..span.start];
            let post_content = &new_content[span.end..];
            new_content = format!("{}{}{}", pre_content, block.as_str(), post_content);
        }

        Ok(new_content)
    }

    fn extract_godbolt_info(info_string: &str, content: &str) -> Option<Godbolt> {
        let info = GodboltMeta::new(info_string)?;

        let codeblock = strip_godbolt_from_codeblock(content, &info);

        Some(Godbolt::new(info, codeblock))
    }

    fn strip_godbolt_from_codeblock(content: &str, info: &GodboltMeta) -> String {
        let start_idx = body_start_index(content);

        format!("```{}\n{}", &info.lang, &content[start_idx..])
            .trim_end()
            .to_string()
    }

    fn body_start_index(content: &str) -> usize {
        let index = content
            .find('\n')
            .map(|idx| idx + 1); // Start with character after newline

        match index {
            None => 0,
            // Check for out of bounds indexes
            Some(idx) => if idx > (content.len() - 1) { 0 } else { idx }
        }
    }

    #[deprecated]
    fn body_end_index(content: &str) -> usize {
        let fchar = content.chars().next_back().unwrap_or('`');
        let num_fchar = content
            .chars()
            .rev()
            .position(|c| c != fchar)
            .unwrap_or_default();

        content.len() - num_fchar
    }
}
