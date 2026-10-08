use crate::types::Line;
use eframe::egui::Pos2;
use rfd::FileDialog;
use std::path::PathBuf;

const DEFAULT_DIR: &str = "files";

fn default_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_DIR)
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
        .set_file_name(format!("untitled.{extension}"))
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

pub fn save_csv(title: &str, default_name: &str, data: &str) -> bool {
    let path = FileDialog::new()
        .set_title(title)
        .set_directory(default_dir())
        .add_filter("CSV", &["csv"])
        .set_file_name(default_name)
        .save_file();
    match path {
        Some(p) => std::fs::write(&p, data).is_ok(),
        None => false,
    }
}

/// Sérialise les points en CSV `x,y`. Les indices des lignes de ce fichier
/// correspondent aux indices utilisés dans `edges_to_csv`.
pub fn points_to_csv(points: &[Pos2]) -> String {
    let mut csv = String::from("x,y\n");
    for p in points {
        csv.push_str(&format!("{},{}\n", p.x, p.y));
    }
    csv
}

/// Sérialise les liaisons en CSV `i,j` (indices des deux sommets reliés,
/// dans l'ordre du fichier exporté par `points_to_csv`).
pub fn edges_to_csv(lines: &[Line]) -> String {
    let mut csv = String::from("i,j\n");
    for &[a, b] in lines {
        csv.push_str(&format!("{},{}\n", a, b));
    }
    csv
}

/// Sérialise les liaisons en CSV `i,j,t` avec le poids `t` de chaque liaison.
pub fn edges_to_csv_with_weights(lines: &[Line], weights: &[f32]) -> String {
    let mut csv = String::from("i,j,t\n");
    for (&[a, b], &t) in lines.iter().zip(weights) {
        csv.push_str(&format!("{},{},{}\n", a, b, t));
    }
    csv
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    #[test]
    fn points_csv_format() {
        let csv = points_to_csv(&[pos2(-20.0, 0.0), pos2(-10.0, 8.660254)]);
        assert_eq!(csv, "x,y\n-20,0\n-10,8.660254\n");
    }

    #[test]
    fn edges_csv_format() {
        let csv = edges_to_csv(&[[0, 1], [1, 2]]);
        assert_eq!(csv, "i,j\n0,1\n1,2\n");
    }

    #[test]
    fn edges_csv_with_weights_format() {
        let csv = edges_to_csv_with_weights(&[[0, 1], [1, 2]], &[1.0, 0.5]);
        assert_eq!(csv, "i,j,t\n0,1,1\n1,2,0.5\n");
    }
}
