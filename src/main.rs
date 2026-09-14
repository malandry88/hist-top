use std::env;
use std::fs;
use std::process::ExitCode;

use histtop::parser::parse_history;
use histtop::stats::{command_frequency, filter_by_min_words};

struct Options {
    path: String,
    top: usize,
    min_words: usize,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut path: Option<String> = None;
    let mut top: usize = 20;
    let mut min_words: usize = 1;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--top" => {
                i += 1;
                let value = args.get(i).ok_or("--top needs a value")?;
                top = value.parse().map_err(|_| "--top must be a positive number")?;
            }
            "--min-words" => {
                i += 1;
                let value = args.get(i).ok_or("--min-words needs a value")?;
                min_words = value
                    .parse()
                    .map_err(|_| "--min-words must be a positive number")?;
            }
            other if path.is_none() => path = Some(other.to_string()),
            other => return Err(format!("unexpected argument: {other}")),
        }
        i += 1;
    }

    let path = path.ok_or("missing path to a shell history file")?;
    Ok(Options { path, top, min_words })
}

/// Guesses a default history file from `$SHELL` and `$HOME` when the user
/// doesn't pass a path explicitly.
fn default_history_path() -> Option<String> {
    let shell = env::var("SHELL").ok()?;
    let home = env::var("HOME").ok()?;
    if shell.ends_with("zsh") {
        Some(format!("{home}/.zsh_history"))
    } else {
        Some(format!("{home}/.bash_history"))
    }
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = env::args().skip(1).collect();

    let args = if raw_args.is_empty() {
        match default_history_path() {
            Some(path) => vec![path],
            None => {
                eprintln!("usage: histtop <history-file> [--top N] [--min-words N]");
                return ExitCode::FAILURE;
            }
        }
    } else {
        raw_args
    };

    let options = match parse_args(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("usage: histtop <history-file> [--top N] [--min-words N]");
            return ExitCode::FAILURE;
        }
    };

    let content = match fs::read_to_string(&options.path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("error reading {}: {}", options.path, err);
            return ExitCode::FAILURE;
        }
    };

    let entries = parse_history(&content);
    let entries = filter_by_min_words(&entries, options.min_words);
    let ranked = command_frequency(&entries);

    for (command, count) in ranked.into_iter().take(options.top) {
        println!("{count:>6}  {command}");
    }

    ExitCode::SUCCESS
}
