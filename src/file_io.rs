use crate::types::Line;
use eframe::egui::Pos2;
use rfd::FileDialog;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const DEFAULT_DIR: &str = "files";

/// Derniers dossiers utilisés dans l'explorateur, mémorisés entre deux lancements.
/// Les fichiers de l'app et les exports CSV ont chacun le leur.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecentDirs {
    pub files: Option<PathBuf>,
    pub csv: Option<PathBuf>,
}

#[derive(Clone, Copy)]
enum DirKind {
    Files,
    Csv,
}

static RECENT_DIRS: Mutex<RecentDirs> = Mutex::new(RecentDirs { files: None, csv: None });

pub fn recent_dirs() -> RecentDirs {
    RECENT_DIRS.lock().map(|dirs| dirs.clone()).unwrap_or_default()
}

pub fn set_recent_dirs(dirs: RecentDirs) {
    if let Ok(mut recent) = RECENT_DIRS.lock() {
        *recent = dirs;
    }
}

/// `files/` s'il existe, sinon le dossier de lancement de l'app.
fn default_dir() -> PathBuf {
    let app_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let files_dir = app_dir.join(DEFAULT_DIR);
    if files_dir.is_dir() {
        files_dir
    } else {
        app_dir
    }
}

/// Dossier d'ouverture de l'explorateur : le dernier utilisé pour ce type de
/// fichier s'il existe encore, sinon le dossier par défaut.
fn dialog_dir(kind: DirKind) -> PathBuf {
    let dirs = recent_dirs();
    let last = match kind {
        DirKind::Files => dirs.files,
        DirKind::Csv => dirs.csv,
    };
    last.filter(|dir| dir.is_dir()).unwrap_or_else(default_dir)
}

/// Mémorise le dossier du fichier choisi dans l'explorateur.
fn remember_dir(kind: DirKind, path: &Path) {
    let (Some(parent), Ok(mut dirs)) = (path.parent(), RECENT_DIRS.lock()) else {
        return;
    };
    let slot = match kind {
        DirKind::Files => &mut dirs.files,
        DirKind::Csv => &mut dirs.csv,
    };
    *slot = Some(parent.to_path_buf());
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
        .set_directory(dialog_dir(DirKind::Files))
        .add_filter("JSON", &[extension, "json"])
        .pick_file()?;
    remember_dir(DirKind::Files, &path);
    let content = std::fs::read_to_string(&path).ok()?;
    let cleaned = clean_json(&content);
    Some((path, cleaned))
}

pub fn save_json(title: &str, extension: &str, data: &str) -> bool {
    let path = FileDialog::new()
        .set_title(title)
        .set_directory(dialog_dir(DirKind::Files))
        .add_filter("JSON", &[extension, "json"])
        .set_file_name(format!("untitled.{extension}"))
        .save_file();
    match path {
        Some(p) => {
            remember_dir(DirKind::Files, &p);
            std::fs::write(&p, data).is_ok()
        }
        None => false,
    }
}

pub fn save_json_path(title: &str, extension: &str, default_name: &str, data: &str) -> bool {
    let path = FileDialog::new()
        .set_title(title)
        .set_directory(dialog_dir(DirKind::Files))
        .add_filter("JSON", &[extension, "json"])
        .set_file_name(default_name)
        .save_file();
    match path {
        Some(p) => {
            remember_dir(DirKind::Files, &p);
            std::fs::write(&p, data).is_ok()
        }
        None => false,
    }
}

/// Demande un nom de base pour un export en plusieurs fichiers CSV.
/// Renvoie le chemin choisi sans l'extension `.csv`, ou `None` si l'utilisateur annule.
pub fn pick_csv_base(title: &str, default_stem: &str) -> Option<PathBuf> {
    FileDialog::new()
        .set_title(title)
        .set_directory(dialog_dir(DirKind::Csv))
        .add_filter("CSV", &["csv"])
        .set_file_name(format!("{default_stem}.csv"))
        .save_file()
        .map(|p| {
            remember_dir(DirKind::Csv, &p);
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv")) {
                p.with_extension("")
            } else {
                p
            }
        })
}

/// Chemin `<base><suffix>.csv`, ex. `fractale` + `_points` -> `fractale_points.csv`.
pub fn csv_path(base: &Path, suffix: &str) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(suffix);
    name.push(".csv");
    PathBuf::from(name)
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
/// dans l'ordre du fichier exporté par `points_to_csv`), ou `i,j,t` si un
/// poids `t` est fourni pour chaque liaison.
pub fn edges_to_csv(lines: &[Line], weights: Option<&[f32]>) -> String {
    let mut csv = String::from(if weights.is_some() { "i,j,t\n" } else { "i,j\n" });
    for (k, &[a, b]) in lines.iter().enumerate() {
        match weights {
            Some(w) => csv.push_str(&format!("{a},{b},{}\n", w[k])),
            None => csv.push_str(&format!("{a},{b}\n")),
        }
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
    fn csv_path_appends_suffix() {
        let path = csv_path(Path::new("files/csv/sierpinski"), "_edges");
        assert_eq!(path, PathBuf::from("files/csv/sierpinski_edges.csv"));
    }

    #[test]
    fn edges_csv_format() {
        let csv = edges_to_csv(&[[0, 1], [1, 2]], None);
        assert_eq!(csv, "i,j\n0,1\n1,2\n");
    }

    #[test]
    fn edges_csv_with_weights_format() {
        let csv = edges_to_csv(&[[0, 1], [1, 2]], Some(&[1.0, 0.5]));
        assert_eq!(csv, "i,j,t\n0,1,1\n1,2,0.5\n");
    }
}
