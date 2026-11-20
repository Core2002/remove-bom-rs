use std::process::ExitCode;

use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

#[derive(StructOpt, Debug)]
#[structopt(name = "remove-bom", about = "Remove the UTF-8 BOM from files")]
struct Opt {
    /// File extensions separated by commas, e.g. `txt,rs` (a leading dot is optional)
    #[structopt(
        short = "e",
        long = "exts",
        value_name = "extensions",
        required = true,
        use_delimiter = true
    )]
    extensions: Vec<String>,
}

fn main() -> ExitCode {
    let opt = Opt::from_args();
    let suffixes = suffixes(&opt.extensions);
    if suffixes.is_empty() {
        eprintln!("error: at least one non-empty file extension is required");
        return ExitCode::FAILURE;
    }

    let mut failures = 0usize;

    let walker = WalkDir::new(".").into_iter();
    for entry in walker.filter_entry(|entry| keep_entry(&suffixes, entry)) {
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

/// Turns raw `--exts` values into file name suffixes such as `.txt`.
///
/// Empty items are dropped, surrounding whitespace is ignored, and a leading
/// dot is tolerated, so `txt`, `.txt` and ` txt ` all mean the same thing.
/// The extensions are used literally — they are not regular expressions.
fn suffixes(extensions: &[String]) -> Vec<String> {
    extensions
        .iter()
        .map(|extension| extension.trim().trim_start_matches('.'))
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!(".{extension}"))
        .collect()
}

/// Keeps every directory so the walk descends into it, and only the files whose
/// name ends with one of `suffixes`.
fn keep_entry(suffixes: &[String], entry: &DirEntry) -> bool {
    if entry.file_type().is_dir() {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    suffixes
        .iter()
        .any(|suffix| name.ends_with(suffix.as_str()))
}
