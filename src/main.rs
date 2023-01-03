use regex::Regex;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path};
use anyhow::{Result, anyhow};
use similar::{ChangeTag, DiffTag, TextDiff};
use clap::{Parser, Subcommand};
use colored::{Color, ColoredString, Colorize};


#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    find: String,
    /// Use $1, $n to specify group replacement. For example, `sd 'foo(\d+)' 'bar$1'` will replace "foo55" with "bar55".
    replace_with: String,
    files: Option<Vec<String>>,

    #[clap(short, long)]
    string_mode: bool,

    #[clap(short, long)]
    force: bool,
}

struct Stylizer(ChangeTag);

impl Stylizer {
    fn style(&self, s: &str) -> ColoredString {
        match self.0 {
            ChangeTag::Delete => s.red(),
            ChangeTag::Insert => s.green(),
            ChangeTag::Equal => s.normal(),
        }
    }

    fn sign(&self) -> ColoredString {
        match self.0 {
            ChangeTag::Delete => "-".red(),
            ChangeTag::Insert => "+".green(),
            ChangeTag::Equal => " ".normal(),
        }
    }
}

fn do_file_replacement(path: &Path, find: &Regex, replace_with: &str, force: bool) -> Result<()> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(());
    };
    let new_contents = find.replace_all(&contents, &*replace_with);
    if new_contents != contents {
        if force {
            fs::write(path, new_contents.as_bytes())?;
            eprintln!("{}: File was changed.", path.display());
        } else {
            let mut stdout = std::io::stdout().lock();
            let diff = TextDiff::from_lines(contents.as_str(), &new_contents);
            println!("--- a/{0}\n+++ b/{0}", path.display());
            for group in diff.grouped_ops(3).iter() {
                for op in group {
                    if op.tag() == DiffTag::Equal {
                        continue;
                    }
                    for change in diff.iter_inline_changes(op) {
                        let stylizer = Stylizer(change.tag());
                        write!(stdout, "{}", stylizer.sign()).unwrap();
                        for (emphasized, value) in change.iter_strings_lossy() {
                            let value = value.to_string();
                            if emphasized {
                                write!(stdout, "{}", stylizer.style(&value)).unwrap();
                            } else {
                                write!(stdout, "{}", value).unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let has_stdin = atty::isnt(atty::Stream::Stdin);

    let find = if cli.string_mode {
        Regex::new(regex::escape(&cli.find).as_str())?
    } else {
        Regex::new(&cli.find).map_err(|e| anyhow!("Tried to parse {:?} as a regex, but failed.\nTry using -s to interpret <FIND> as a string, or fix your regex.\n\n{}", &cli.find, e))?
    };

    let replacer = Regex::new(r"(^|[^\\])($)(\d+)").unwrap();
    let replace_with = replacer.replace_all(&cli.replace_with, r"$1$${$3}").to_string();
    if let Some(files) = &cli.files {
        for file in files {
            do_file_replacement(Path::new(file), &find, &replace_with, cli.force)?;
        }
    } else if has_stdin {
        let replace_with = replace_with.green().to_string();
        for line in std::io::stdin().lock().lines() {
            let line = line?;
            let line = find.replace_all(&line, &replace_with);
            println!("{}", line);
        }
    } else {
        // recurse through the current directory
        for entry in ignore::Walk::new("./").filter_map(|r| r.ok()).filter(|e| e.path().is_file()) {
            do_file_replacement(entry.path(), &find, &replace_with, cli.force)?;
        }
    }
    Ok(())
}
