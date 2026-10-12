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

/// Message lisible pour une erreur IO sur `path`.
fn io_error_message(action: &str, path: &Path, err: &std::io::Error) -> String {
    use std::io::ErrorKind;
    let cause = match err.kind() {
        ErrorKind::NotFound => "fichier ou dossier introuvable".to_string(),
        ErrorKind::PermissionDenied => "permissions insuffisantes".to_string(),
        ErrorKind::InvalidData => "contenu invalide (encodage UTF-8 attendu)".to_string(),
        ErrorKind::StorageFull => "disque plein".to_string(),
        ErrorKind::ReadOnlyFilesystem => "système de fichiers en lecture seule".to_string(),
        ErrorKind::IsADirectory => "le chemin désigne un dossier".to_string(),
        ErrorKind::InvalidFilename => "nom de fichier invalide".to_string(),
        _ => err.to_string(),
    };
    format!("Impossible {action} {} : {cause}", path.display())
}

/// Ouvre un fichier JSON choisi dans l'explorateur.
/// Renvoie `Ok(None)` si l'utilisateur annule, `Err` si la lecture échoue.
pub fn open_json(title: &str, extension: &str) -> Result<Option<(PathBuf, String)>, String> {
    let Some(path) = FileDialog::new()
        .set_title(title)
        .set_directory(dialog_dir(DirKind::Files))
        .add_filter("JSON", &[extension, "json"])
        .pick_file()
    else {
        return Ok(None);
    };
    remember_dir(DirKind::Files, &path);
    let content = std::fs::read_to_string(&path).map_err(|e| io_error_message("de lire", &path, &e))?;
    Ok(Some((path, content)))
}

pub fn save_json(title: &str, extension: &str, data: &str) -> Result<Option<PathBuf>, String> {
    save_json_path(title, extension, &format!("untitled.{extension}"), data)
}

/// Enregistre `data` à l'emplacement choisi dans l'explorateur.
/// Renvoie `Ok(None)` si l'utilisateur annule, `Err` si l'écriture échoue.
pub fn save_json_path(title: &str, extension: &str, default_name: &str, data: &str) -> Result<Option<PathBuf>, String> {
    let Some(path) = FileDialog::new()
        .set_title(title)
        .set_directory(dialog_dir(DirKind::Files))
        .add_filter("JSON", &[extension, "json"])
        .set_file_name(default_name)
        .save_file()
    else {
        return Ok(None);
    };
    remember_dir(DirKind::Files, &path);
    std::fs::write(&path, data).map_err(|e| io_error_message("d'écrire", &path, &e))?;
    Ok(Some(path))
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
    fn io_error_message_explains_cause() {
        let err = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let msg = io_error_message("d'écrire", Path::new("files/a.ftlfw"), &err);
        assert_eq!(msg, "Impossible d'écrire files/a.ftlfw : permissions insuffisantes");
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
