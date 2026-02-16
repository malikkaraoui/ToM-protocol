use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Global JSONL file writer. Set once in main(), read by emit().
static JSONL_WRITER: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();

pub struct OutputPaths {
    pub jsonl: PathBuf,
    pub log: PathBuf,
}

/// Build output file paths, ensuring the directory exists.
/// Pattern: `<dir>/<name>_<mode>_<YYYYMMDD-HHMMSS>.{jsonl,log}`
pub fn resolve_output_paths(dir: &Path, name: &str, mode: &str) -> io::Result<OutputPaths> {
    fs::create_dir_all(dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let base = format!("{name}_{mode}_{timestamp}");

    let jsonl = find_unique_path(dir, &base, "jsonl");
    let log = find_unique_path(dir, &base, "log");

    Ok(OutputPaths { jsonl, log })
}

/// Find a path that does not yet exist, appending _2, _3... if needed.
fn find_unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let candidate = dir.join(format!("{base}.{ext}"));
    if !candidate.exists() {
        return candidate;
    }
    for i in 2.. {
        let candidate = dir.join(format!("{base}_{i}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

/// Initialize the global JSONL file writer.
pub fn init_jsonl_writer(path: &Path) -> io::Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let writer = BufWriter::new(file);
    JSONL_WRITER
        .set(Mutex::new(writer))
        .map_err(|_| io::Error::new(io::ErrorKind::AlreadyExists, "JSONL writer already set"))?;
    Ok(())
}

/// Write a JSONL line to the file (if initialized). Called from emit().
pub fn write_jsonl_line(line: &str) {
    if let Some(writer) = JSONL_WRITER.get() {
        if let Ok(mut w) = writer.lock() {
            let _ = writeln!(w, "{line}");
            let _ = w.flush();
        }
    }
}
