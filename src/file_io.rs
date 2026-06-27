use rfd::FileDialog;
use std::path::{Path, PathBuf};

const DEFAULT_DIR: &str = "/home/cesar/Bureau/Projets/fractal-wizard/files";

fn default_dir() -> &'static Path {
    Path::new(DEFAULT_DIR)
}

fn clean_json(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;
    while i < bytes.len() {
        if escape {
            escape = false;
        } else if bytes[i] == b'\\' {
            escape = true;
        } else if bytes[i] == b'"' {
            in_string = !in_string;
        }
        if !in_string && bytes[i] == b',' {
            let mut j = i + 1;
            while j < bytes.len()
                && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r')
            {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b']' || bytes[j] == b'}') {
                i += 1;
                continue;
            }
        }
        output.push(bytes[i] as char);
        i += 1;
    }
    output
}

pub fn open_json(title: &str, extension: &str) -> Option<(PathBuf, String)> {
    let path = FileDialog::new()
        .set_title(title)
        .set_directory(default_dir())
        .add_filter("JSON", &[extension, "json"])
        .pick_file()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let cleaned = clean_json(&content);
    Some((path, cleaned))
}

pub fn save_json(title: &str, extension: &str, data: &str) -> bool {
    let path = FileDialog::new()
        .set_title(title)
        .set_directory(default_dir())
        .add_filter("JSON", &[extension, "json"])
        .set_file_name(&format!("untitled.{}", extension))
        .save_file();
    match path {
        Some(p) => std::fs::write(&p, data).is_ok(),
        None => false,
    }
}

pub fn save_json_path(title: &str, extension: &str, default_name: &str, data: &str) -> bool {
    let path = FileDialog::new()
        .set_title(title)
        .set_directory(default_dir())
        .add_filter("JSON", &[extension, "json"])
        .set_file_name(default_name)
        .save_file();
    match path {
        Some(p) => std::fs::write(&p, data).is_ok(),
        None => false,
    }
}
