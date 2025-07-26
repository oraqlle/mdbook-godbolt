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
        .subcommand(Command::new("install"))
}

fn main() {
    let matches = make_cli().get_matches();

    let preprocessor = GodboltPreprocessor::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Some(_) = matches.subcommand_matches("install") {
        if let Err(e) = install::handle_install() {
            eprintln!("Error installing mdbook-godbolt: {e}");
        } else {
            println!("Installed mdbook-godbolt preprocessor");
        }
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

mod install {
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };

    use anyhow::{Context, Result};
    use toml_edit::{DocumentMut, Item, Table};

    const ASSETS_VER: &str = include_str!("../assets/VERSION");

    const GODBOLT_BOOKJS: (&str, &[u8]) = ("book.js", include_bytes!("../assets/book.js"));

    pub fn handle_install() -> Result<()> {
        let proj_dir = PathBuf::from(".");
        let config = proj_dir.join("book.toml");

        let toml = fs::read_to_string(&config)
            .with_context(|| format!("can't read configuration file '{}'", config.display()))?;

        let mut doc = toml
            .parse::<DocumentMut>()
            .context("configuration is not valid TOML")?;

        // Inject preprocessor config into in-memory TOML config
        if let Err(_) = inject_preprocessor(&mut doc) {
            eprintln!("Error injecting preprocessor config in `book.toml'");
        };

        let path = proj_dir.join("theme").components().collect::<PathBuf>();

        if !path.exists() {
            fs::create_dir(&path)?;
        }

        let filepath = &path.join(GODBOLT_BOOKJS.0);

        println!("Copying `{}' to '{}'", GODBOLT_BOOKJS.0, filepath.display());

        let mut file = File::create(&filepath).context("can't open file for writing")?;
        file.write_all(GODBOLT_BOOKJS.1)
            .context("can't write content to file")?;

        // Create new TOML config and write to disk
        let new_toml = doc.to_string();

        if new_toml != toml {
            println!("Saving changed configuration to `{}'", config.display());

            let mut file =
                File::create(config).context("can't open configuration file for writing.")?;

            file.write_all(new_toml.as_bytes())
                .context("can't write configuration")?;
        } else {
            eprintln!("Configuration `{}' already up to date", config.display());
        }

        Ok(())
    }

    fn inject_preprocessor(doc: &mut DocumentMut) -> Result<(), ()> {
        let doc = doc.as_table_mut();

        let pre_table = doc
            .entry("preprocessor")
            .or_insert(Item::Table(Table::default()))
            .as_table_mut()
            .ok_or(())?;

        pre_table.set_dotted(true);

        let gd_table = pre_table
            .entry("godbolt")
            .or_insert(Item::Table(Table::default()));

        gd_table["command"] = toml_edit::value("mdbook-godbolt");

        gd_table["assets_version"] = toml_edit::value(
            toml_edit::Value::from(ASSETS_VER.trim())
                .decorated(" ", " # do not edit: managed by `mdbook-godbolt install`"),
        );

        Ok(())
    }
}

mod libgodbolt {
    use std::collections::HashMap;

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

                result = Some(preprocess(&ch.content).map(|md| ch.content = md));
            });

            // If an error occurred return book as is
            result.unwrap_or(Ok(())).map(|_| book)
        }
    }

    fn parse_info_str(info_string: &str) -> Option<HashMap<&str, &str>> {
        if info_string.contains("godbolt") {
            Some(
                info_string
                    .split(',')
                    .filter_map(|chunk| {
                        if chunk.starts_with("godbolt-compiler:") {
                            Some(("compiler", &chunk[17..]))
                        } else if chunk.starts_with("godbolt-flags:") {
                            Some(("flags", &chunk[14..]))
                        } else if chunk.starts_with("godbolt") {
                            None
                        } else {
                            Some(("lang", chunk))
                        }
                    })
                    .collect(),
            )
        } else {
            None
        }
    }

    #[derive(Debug)]
    struct Godbolt<'a> {
        codeblock: String,
        compiler: Option<&'a str>,
        flags: Option<&'a str>,
    }

    impl<'a> Godbolt<'a> {
        pub(crate) fn new(info_string: &'a str, content: &str) -> Option<Self> {
            let info = parse_info_str(info_string)?;

            let lang = info.get("lang")?;
            let compiler = info.get("compiler").map(|v| &**v);
            let flags = info.get("flags").map(|v| &**v);
            let codeblock = strip_godbolt_from_codeblock(content, &lang);

            eprintln!("{:?}", &info);

            Some(Self {
                codeblock,
                compiler,
                flags
            })
        }

        pub(crate) fn add_godbolt_pre(self) -> String {
            let html = mdbook::utils::render_markdown(&self.codeblock, false);

            let code_start_idx = html.find("<code").unwrap();
            let code_end_idx = html.find("</code>").unwrap() + 7;
            let code_block = &html[code_start_idx..code_end_idx];

            let compiler = &self.compiler.map_or(String::new(), |txt| {
                format!(" data-godbolt-compiler=\"{}\"", txt)
            });

            let flags = &self.flags.map_or(String::new(), |txt| {
                format!(" data-godbolt-flags=\"{}\"", txt)
            });

            eprintln!("{:?}", &self);
            eprintln!("{:?}", self.compiler.is_some());
            eprintln!("{:?}", self.flags.is_some());

            format!(
                "<pre><pre class=\"godbolt\"{}{}>{}</pre></pre>",
                compiler, flags, code_block
            )
        }
    }

    fn preprocess(content: &str) -> MdBookResult<String> {
        // Get markdown parsing events as iterator
        let events = Parser::new_ext(content, Options::empty());

        let mut godbolt_blocks = vec![];

        // Iterate through events finding codeblocks
        for (event, span) in events.into_offset_iter() {
            if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info_string))) = event.clone()
            {
                let code_content = &content[span.start..span.end];

                let godbolt = match Godbolt::new(info_string.as_ref(), code_content) {
                    Some(gbolt) => gbolt,
                    None => continue,
                };

                // Adds HTML data around codeblock content
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

    fn strip_godbolt_from_codeblock(content: &str, lang: &str) -> String {
        let start_idx = body_start_index(content);

        format!("```{}\n{}", &lang, &content[start_idx..])
            .trim_end()
            .to_string()
    }

    fn body_start_index(content: &str) -> usize {
        let index = content.find('\n').map(|idx| idx + 1); // Start with character after newline

        match index {
            None => 0,
            // Check for out of bounds indexes
            Some(idx) => {
                if idx > (content.len() - 1) {
                    0
                } else {
                    idx
                }
            }
        }
    }
}
