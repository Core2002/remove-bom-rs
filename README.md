# remove-bom

**English** | [简体中文](README.zh-CN.md)

A small command-line tool — and a tiny library — that strips the UTF-8 BOM from files, recursively.

A UTF-8 BOM (`EF BB BF`) at the start of a file is perfectly legal, but it regularly breaks
things that look at the first bytes of a file: shebang lines in scripts, CSV headers, Makefiles,
and assorted parsers. `remove-bom` walks a directory tree and drops that prefix from the files
you care about.

## Features

- **Recursive** — walks the current directory and every subdirectory below it.
- **Selective** — only touches files whose extension you list (`-e txt,rs`).
- **Idempotent** — files without a BOM are left byte-for-byte untouched.
- **Safe by default** — rewrites go through a temporary file and an atomic `rename`; permissions
  are preserved and a failed run never leaves a file half-written.
- **Honest failures** — I/O problems are reported per file, the rest of the tree is still
  processed, and the exit status reflects whether anything failed.
- **Library too** — `remove_bom::remove_bom(path)` does the work for one file.

## Installation

Requires Rust 1.85 or newer (the crate is on `edition = "2024"`).

```bash
git clone https://github.com/Core2002/remove-bom-rs
cd remove-bom-rs

# install into ~/.cargo/bin
cargo install --path .

# …or just build it
cargo build --release   # binary at target/release/remove-bom
```

> The `remove-bom` crate on crates.io is the original 0.1.0 release from 2018. `cargo install
> remove-bom` will install that older snapshot, not this source tree — build from source as shown
> above for the current code.

## Usage

```text
remove-bom --exts <extensions>...
```

| Option | Description |
| --- | --- |
| `-e, --exts <extensions>...` | **Required.** File extensions, separated by commas (`-e txt,rs`) or given more than once (`-e txt -e rs`). A leading dot is optional: `txt` and `.txt` are the same. |
| `-h, --help` | Print help. |
| `-V, --version` | Print version. |

Examples:

```bash
# Strip BOMs from every .txt and .rs file under the current directory
remove-bom -e txt,rs

# Same thing, long form
remove-bom --exts=html,css,js

# Repeating the flag works as well
remove-bom -e .txt -e .rs
```

A short session:

```console
$ printf '\xef\xbb\xbfhello\n' > a.txt
$ remove-bom -e txt
./a.txt
$ od -An -tx1 a.txt
 68 65 6c 6c 6f 0a
```

`remove-bom` exits with status `0` when every visited file was processed, and `1` when at least one
file or directory could not be handled. Failures are printed on stderr, one line each, and never
abort the walk early.

## What it does to each file

1. Directories are always kept so the walk can descend; a file is selected when its name ends with
   `.<extension>` for one of the listed extensions (a literal, case-sensitive suffix match, so
   `tar.gz` works too).
2. The first three bytes are read. If they are not `EF BB BF`, the file is left untouched — but it
   is still printed, since it was visited.
3. Otherwise the remaining bytes are copied into a uniquely named temporary file in the same
   directory (`.NAME.remove-bom-<pid>-<n>.tmp`), the original permissions are copied onto it, and
   the temporary file is renamed over the original. The rename is atomic, so the file is never
   observed half-written. If anything fails, the temporary file is deleted and the original stays
   intact.

Files are never named `<file>.tmp`, so an unrelated file that happens to use that name is safe.

## Behaviour and limits

Worth knowing before you run it on something you care about:

- It always operates on the **current working directory**; there is no path argument. `cd` first.
- Extension matching is **case-sensitive**: `-e txt` does not match `README.TXT`.
- Extensions are treated as **literal text**, never as a pattern: `-e 'txt|rs'` looks for files
  ending in `.txt|rs` and matches nothing.
- An extension list that is empty after trimming is rejected instead of silently doing nothing.
- **No ignore rules.** Hidden files, `.git/`, `target/` and similar directories are walked too.
- Symbolic links are not followed, and symlinked files are not modified.
- Files without a BOM are **printed but not modified** — output is a list of visited files, not a
  list of changed files.
- The rewrite is **in place**: no dry-run mode and no backup. The file is replaced by a new inode,
  so hard links are broken and extended attributes/ACLs are not carried over (permissions are).
- A crash or `SIGKILL` mid-rewrite can leave a hidden `.NAME.remove-bom-<pid>-<n>.tmp` file behind;
  the original is intact in that case, and the leftover is safe to delete.
- I/O errors (permission denied, unreadable directory, …) are reported on stderr and produce exit
  status `1`; the remaining files are still processed.
- File names that are not valid UTF-8 are matched through a lossy decoding, so a file whose name
  ends in a plain ASCII extension is still handled.

Given the above, run it on a clean working tree, and check `git diff` afterwards — a BOM removal
shows up as a one-line change at the top of each affected file.

## As a library

```rust
use remove_bom::remove_bom;

fn main() -> std::io::Result<()> {
    // Ok(true)  => a BOM was found and stripped
    // Ok(false) => the file did not start with a BOM and was not touched
    if remove_bom("a.txt")? {
        println!("stripped the BOM from a.txt");
    }
    Ok(())
}
```

`remove_bom` accepts anything that implements `AsRef<Path>`. Since 0.2.0 it returns
`io::Result<bool>` instead of panicking and returning a bare `bool`; on error (missing file,
permission denied, …) the temporary file has already been cleaned up and the original is unchanged.

## Development

```bash
cargo build
cargo build --release
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test          # 9 unit tests covering the rewrite and its failure modes
```

## License

BSD-2-Clause — see [LICENSE](LICENSE). Copyright (c) 2018, hyq.

## Credits

Originally written by [hyq](https://github.com/hyqhyq3) as
[hyqhyq3/remove-bom-rs](https://github.com/hyqhyq3/remove-bom-rs); this repository keeps the tool
building on current Rust toolchains (edition 2024, refreshed dependencies, `rustfmt`-formatted) and
fixes the error handling and rewrite path.
