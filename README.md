# remove-bom

A small command-line tool — and a tiny library — that strips the UTF-8 BOM from files, recursively.

A UTF-8 BOM (`EF BB BF`) at the start of a file is perfectly legal, but it regularly breaks
things that look at the first bytes of a file: shebang lines in scripts, CSV headers, Makefiles,
and assorted parsers. `remove-bom` walks a directory tree and drops that prefix from the files
you care about.

## Features

- **Recursive** — walks the current directory and every subdirectory below it.
- **Selective** — only touches files whose extension you list (`-e txt,rs,md`).
- **Idempotent** — files without a BOM are left byte-for-byte untouched.
- **Verbose by default** — prints every matched path as it goes, so you can see what was visited.
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

> The `remove-bom` crate on crates.io is the original 0.1.0 release from 2018. `cargo install remove-bom`
> will install that older snapshot, not this source tree — build from source as shown above for the
> current code.

## Usage

```text
remove-bom --exts <extensions>
```

| Option | Description |
| --- | --- |
| `-e, --exts <extensions>` | **Required.** Comma-separated file extensions, e.g. `txt,rs,md`. |
| `-h, --help` | Print help. |
| `-V, --version` | Print version. |

Examples:

```bash
# Strip BOMs from every .txt and .rs file under the current directory
remove-bom -e txt,rs

# Same thing, long form
remove-bom --exts=html,css,js
```

A short session:

```console
$ printf '\xef\xbb\xbfhello\n' > a.txt
$ remove-bom -e txt
./a.txt
$ od -An -tx1 a.txt
 68 65 6c 6c 6f 0a
```

## What it does to each file

1. A file is selected when its name ends with `.<extension>` for one of the listed extensions.
2. The first three bytes are read. If they are not `EF BB BF`, the file is left untouched — but it is
   still printed, since it was visited.
3. Otherwise the file is rewritten: it is renamed to `<file>.tmp`, a fresh `<file>` is created, the
   bytes after the BOM are copied over, and the `.tmp` file is deleted.

Matching is done with a regular expression built from the extension list, so semantics are a
case-sensitive suffix match.

## Behaviour and limits

Worth knowing before you run it on something you care about:

- It always operates on the **current working directory**; there is no path argument. `cd` first.
- Extension matching is **case-sensitive**: `-e txt` does not match `README.TXT`.
- Extensions are spliced into a regex, so keep them plain (letters, digits); regex metacharacters in
  an extension will change the match.
- **No ignore rules.** Hidden files, `.git/`, `target/` and similar directories are walked too.
- Symbolic links are not followed, and symlinked files are not modified.
- Files without a BOM are **printed but not modified** — output is a list of visited files, not a
  list of changed files.
- The rewrite is **in place**: no dry-run mode and no backup. A sibling `<file>.tmp` is used while
  working, and a pre-existing file with exactly that name will be overwritten and then deleted.
- It is **not crash-safe**: if the process is interrupted between the rename and the final delete,
  the original content is left sitting at `<file>.tmp`.
- It **panics** (exit code 101) when a matched file cannot be opened — for example on a permission
  error — instead of skipping it.
- File names must be valid UTF-8; paths that are not are silently skipped.

Given the above, run it on a clean working tree, and check `git diff` afterwards — a BOM removal
shows up as a one-line change at the top of each affected file.

## As a library

```rust
use remove_bom::remove_bom;

fn main() {
    // Strips the BOM from a.txt; a file without a BOM comes back unchanged.
    // Returns true on success — I/O failures panic rather than returning false.
    assert!(remove_bom("a.txt"));
}
```

## Development

```bash
cargo build       # debug build
cargo build --release
cargo fmt         # formatting (rustfmt)
cargo test        # there are no tests yet
```

`cargo clippy` currently reports `clippy::unused_io_amount` for the three-byte read in
`src/lib.rs` — a known cleanup rather than an intended omission.

## License

BSD-2-Clause — see [LICENSE](LICENSE). Copyright (c) 2018, hyq.

## Credits

Originally written by [hyq](https://github.com/hyqhyq3) as
[hyqhyq3/remove-bom-rs](https://github.com/hyqhyq3/remove-bom-rs); this repository keeps the tool
building on current Rust toolchains (edition 2024, refreshed dependencies, `rustfmt`-formatted).
