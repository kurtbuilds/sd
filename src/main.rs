use regex::Regex;
use std::fs;
use std::io::BufRead;
use std::path::{Path};
use anyhow::{Result, anyhow};
use similar::{ChangeTag, TextDiff};
use clap::{Parser, Subcommand};


#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    find: String,
    replace_with: String,
    files: Option<Vec<String>>,

    #[clap(short, long)]
    string_mode: bool,

    #[clap(short, long)]
    dry: bool,

}

#[derive(Parser)]
struct AbsoluteToRelative {
    #[clap(long)]
    dry: bool,
    path: String,
}

#[derive(Parser)]
struct RelativeToAbsolute {
    #[clap(long)]
    dry: bool,
    fpath: String,
}

#[derive(Subcommand)]
enum Command {
    AbsoluteToRelative(AbsoluteToRelative),
    RelativeToAbsolute(RelativeToAbsolute),
}


fn do_file_replacement(path: &Path, find: &Regex, replace_with: &str, dry: bool) -> Result<()> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(());
    };
    let new_contents = find.replace_all(&contents, &*replace_with);
    if new_contents != contents {
        if dry {
            let diff = TextDiff::from_lines(contents.as_str(), &new_contents);
            println!("--- a/{0}\n+++ b/{0}", path.display());
            for change in diff.iter_all_changes() {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => continue,
                };
                print!("{}{}", sign, change);
            }
        } else {
            fs::write(path, new_contents.as_bytes())?;
            eprintln!("{}: File was changed.", path.display());
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

    if let Some(files) = &cli.files {
        for file in files {
            do_file_replacement(Path::new(file), &find, &cli.replace_with, cli.dry)?;
        }
    } else if has_stdin {
        for line in std::io::stdin().lock().lines() {
            let line = line?;
            let line = find.replace_all(&line, &cli.replace_with);
            println!("{}", line);
        }
    } else {
        // recurse through the current directory
        for entry in ignore::Walk::new("./").filter_map(|r| r.ok()).filter(|e| e.path().is_file()) {
            do_file_replacement(entry.path(), &find, &cli.replace_with, cli.dry)?;
        }
    }
    Ok(())
}