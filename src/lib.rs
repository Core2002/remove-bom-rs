use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// The UTF-8 byte order mark.
const UTF8_BOM: [u8; 3] = [0xef, 0xbb, 0xbf];

/// Upper bound on the number of temporary file names tried for one rewrite.
const MAX_TEMP_ATTEMPTS: u32 = 1_000;

/// Removes the UTF-8 BOM from the file at `path`.
///
/// Returns `Ok(true)` when a BOM was found and stripped, and `Ok(false)` when
/// the file does not start with one — in that case the file is not touched at
/// all. I/O problems (unreadable file, unwritable directory, …) are reported as
/// errors instead of panicking.
///
/// The new content is written to a uniquely named temporary file next to
/// `path`, then moved into place with an atomic `rename`. Readers therefore
/// never observe a half-written file, the original is not destroyed before the
/// replacement is complete, and permissions are carried over to the new file.
pub fn remove_bom<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let path = path.as_ref();
    let mut source = File::open(path)?;
    if !starts_with_bom(&mut source)? {
        return Ok(false);
    }

    let permissions = source.metadata()?.permissions();
    let (mut temp, temp_path) = create_temp_file(path)?;
    // Deletes the temporary file again unless the rename below succeeded.
    let mut temp_path = TempPath::new(temp_path);

    source.seek(SeekFrom::Start(UTF8_BOM.len() as u64))?;
    io::copy(&mut source, &mut temp)?;
    temp.flush()?;
    fs::set_permissions(temp_path.as_ref(), permissions)?;
    drop(temp);

    fs::rename(temp_path.as_ref(), path)?;
    temp_path.keep();
    Ok(true)
}

/// Checks whether the first bytes read from `reader` are the UTF-8 BOM.
///
/// A file shorter than the BOM cannot start with one; short reads are handled
/// explicitly so that a partial read is never mistaken for "no BOM".
fn starts_with_bom(reader: &mut impl Read) -> io::Result<bool> {
    let mut buf = [0u8; UTF8_BOM.len()];
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => return Ok(false),
            Ok(read) => filled += read,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
    Ok(buf == UTF8_BOM)
}

/// Creates a new, uniquely named temporary file in the directory of `path`.
fn create_temp_file(path: &Path) -> io::Result<(File, PathBuf)> {
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_owned());
    let pid = std::process::id();

    for attempt in 0..MAX_TEMP_ATTEMPTS {
        let candidate = directory.join(format!(".{name}.remove-bom-{pid}-{attempt}.tmp"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((file, candidate)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not find an unused temporary file name",
    ))
}

/// Deletes a temporary file on drop, unless [`TempPath::keep`] was called.
struct TempPath {
    path: PathBuf,
    keep: bool,
}

impl TempPath {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    /// Marks the file as successfully moved into place.
    fn keep(&mut self) {
        self.keep = true;
    }
}

impl AsRef<Path> for TempPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    /// Self-cleaning temporary directory, so the tests need no extra crates.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            let path =
                std::env::temp_dir().join(format!("remove-bom-tests-{}-{id}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            path
        }

        fn read(&self, name: &str) -> Vec<u8> {
            fs::read(self.0.join(name)).unwrap()
        }

        fn entries(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(&self.0)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn removes_leading_bom() {
        let dir = TempDir::new();
        let path = dir.write("a.txt", b"\xef\xbb\xbfhello\n");

        assert!(remove_bom(&path).unwrap());
        assert_eq!(dir.read("a.txt"), b"hello\n");
    }

    #[test]
    fn leaves_file_without_bom_untouched() {
        let dir = TempDir::new();
        let path = dir.write("a.txt", b"hello\n");

        assert!(!remove_bom(&path).unwrap());
        assert_eq!(dir.read("a.txt"), b"hello\n");
    }

    #[test]
    fn does_not_treat_a_truncated_bom_as_a_bom() {
        let dir = TempDir::new();
        let path = dir.write("short.txt", b"\xef\xbb");

        assert!(!remove_bom(&path).unwrap());
        assert_eq!(dir.read("short.txt"), b"\xef\xbb");
    }

    #[test]
    fn handles_empty_and_tiny_files() {
        let dir = TempDir::new();
        let empty = dir.write("empty.txt", b"");
        let tiny = dir.write("tiny.txt", b"a");

        assert!(!remove_bom(&empty).unwrap());
        assert!(!remove_bom(&tiny).unwrap());
        assert_eq!(dir.read("empty.txt"), b"");
        assert_eq!(dir.read("tiny.txt"), b"a");
    }

    #[test]
    fn removes_a_bom_only_file() {
        let dir = TempDir::new();
        let path = dir.write("bom.txt", b"\xef\xbb\xbf");

        assert!(remove_bom(&path).unwrap());
        assert_eq!(dir.read("bom.txt"), b"");
    }

    #[test]
    fn reports_a_missing_file_as_an_error() {
        let dir = TempDir::new();
        let missing = dir.0.join("missing.txt");

        let err = remove_bom(&missing).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn keeps_an_unrelated_file_that_looks_like_a_temp_file() {
        let dir = TempDir::new();
        dir.write("a.txt", b"\xef\xbb\xbfdata\n");
        dir.write("a.txt.tmp", b"precious\n");

        assert!(remove_bom(dir.0.join("a.txt")).unwrap());
        assert_eq!(dir.read("a.txt"), b"data\n");
        assert_eq!(dir.read("a.txt.tmp"), b"precious\n");
    }

    #[test]
    fn leaves_no_temporary_files_behind() {
        let dir = TempDir::new();
        dir.write("a.txt", b"\xef\xbb\xbfdata\n");

        remove_bom(dir.0.join("a.txt")).unwrap();
        assert_eq!(dir.entries(), vec!["a.txt".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn preserves_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new();
        let path = dir.write("a.txt", b"\xef\xbb\xbfdata\n");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        assert!(remove_bom(&path).unwrap());
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }
}
