extern crate structopt;

extern crate regex;
extern crate remove_bom;
extern crate walkdir;

use std::process::ExitCode;

use regex::Regex;
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

#[derive(StructOpt, Debug)]
#[structopt(name = "remove-bom")]
struct Opt {
    #[structopt(
        short = "e",
        long = "exts",
        help = "file extensions separated by comma (,)"
    )]
    extensions: String,
}

fn split_by_comma(s: &str) -> Vec<&str> {
    s.split(',').collect()
}

fn check_file(re: &Regex, x: &DirEntry) -> bool {
    if x.file_type().is_dir() {
        return true;
    }
    x.file_name().to_str().is_some_and(|name| re.is_match(name))
}

fn build_regex(extensions: &[&str]) -> Regex {
    let mut restr = extensions
        .iter()
        .fold(String::new(), |acc, &x| acc + r"\." + x + "$|");
    restr.pop();
    Regex::new(&restr).unwrap()
}

fn main() -> ExitCode {
    let opt = Opt::from_args();
    let extensions = split_by_comma(&opt.extensions);
    let re = build_regex(&extensions);
    let mut failures = 0usize;

    let walker = WalkDir::new(".").into_iter();
    for entry in walker.filter_entry(|x| check_file(&re, x)) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("error: {err}");
                failures += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        match remove_bom::remove_bom(entry.path()) {
            Ok(_) => println!("{}", entry.path().display()),
            Err(err) => {
                eprintln!("error: {}: {err}", entry.path().display());
                failures += 1;
            }
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
