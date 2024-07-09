use std::path::{Path, PathBuf};

use anyhow::Context;
use mdbook::book::{Book, Chapter};
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use mdbook::BookItem;
use regex::Regex;
use walkdir::WalkDir;

use crate::goal::{format_team_asks, team_asks_in_input};

pub struct GoalPreprocessor;

impl Preprocessor for GoalPreprocessor {
    fn name(&self) -> &str {
        "mdbook-goals"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> anyhow::Result<Book> {
        let this = GoalPreprocessorWithContext::new(ctx)?;
        for section in &mut book.sections {
            this.process_book_item(section)?;
        }
        Ok(book)
    }
}

pub struct GoalPreprocessorWithContext<'c> {
    team_asks: Regex,
    ctx: &'c PreprocessorContext,
}

impl<'c> GoalPreprocessorWithContext<'c> {
    pub fn new(ctx: &'c PreprocessorContext) -> anyhow::Result<Self> {
        Ok(GoalPreprocessorWithContext {
            ctx,
            team_asks: Regex::new(r"<!-- TEAM ASKS -->")?,
        })
    }

    fn process_book_item(&self, book_item: &mut BookItem) -> anyhow::Result<()> {
        match book_item {
            BookItem::Chapter(chapter) => {
                self.replace_team_asks(chapter)?;

                for sub_item in &mut chapter.sub_items {
                    self.process_book_item(sub_item)?;
                }

                Ok(())
            }

            BookItem::Separator => Ok(()),

            BookItem::PartTitle(_) => Ok(()),
        }
    }

    /// Look for `<!-- TEAM ASKS -->` in the chapter content and replace it with the team asks.
    fn replace_team_asks(&self, chapter: &mut Chapter) -> anyhow::Result<()> {
        eprintln!("replace_team_asks(chapter.path={:?})", chapter.path);

        let Some(m) = self.team_asks.find(&chapter.content) else {
            return Ok(());
        };
        let range = m.range();

        let Some(path) = &chapter.path else {
            anyhow::bail!("found `<!-- TEAM ASKS -->` but chapter has no path")
        };

        let mut asks_of_any_team = vec![];
        let markdown_files = self.markdown_files(path)?;
        for (input, link_path) in &markdown_files {
            asks_of_any_team.extend(
                team_asks_in_input(input, link_path)
                    .with_context(|| format!("extracting asks from `{}`", input.display()))?,
            );
        }

        let format_team_asks = format_team_asks(&asks_of_any_team)?;

        chapter.content.replace_range(range, &format_team_asks);

        Ok(())
    }

    fn markdown_files(&self, chapter_path: &Path) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
        let chapter_path = self.ctx.config.book.src.join(chapter_path);
        let parent_path = chapter_path.parent().unwrap();

        let mut files = vec![];
        for entry in WalkDir::new(parent_path) {
            let entry = entry?;

            if entry.file_type().is_file() && entry.path().extension() == Some("md".as_ref()) {
                files.push((
                    entry.path().to_path_buf(),
                    entry
                        .path()
                        .strip_prefix(parent_path)
                        .unwrap()
                        .to_path_buf(),
                ));
            }
        }
        Ok(files)
    }
}