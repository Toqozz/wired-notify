use crate::config::Config;
use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn get_icon_dirs() -> Vec<String> {
    let mut icon_dirs = vec![
        "/usr/share/icons".to_string(),
        "/usr/local/share/icons".to_string(),
        expand_tilde("~/.icons"),
        expand_tilde("~/.local/share/icons"),
        "/var/lib/flatpak/exports/share/icons".to_string(),
    ];

    if let Ok(xdg_data_dirs) = env::var("XDG_DATA_DIRS") {
        for p in xdg_data_dirs.split(':') {
            let p = format!("{}/icons", p.trim_end_matches('/'));
            icon_dirs.push(p);
        }
    }

    icon_dirs
}

// Build the theme inheritance chain by parsing `Inherits=` from index.theme files.
// Always ends with hicolor as the final fallback per the freedesktop spec.
pub fn build_theme_chain(theme: &str) -> Vec<String> {
    // Parse the `Inherits=` line from a theme's index.theme file.
    fn parse_inherits(theme: &str, icon_dirs: &Vec<String>) -> Option<Vec<String>> {
        for base_dir in icon_dirs {
            let index_path = Path::new(base_dir).join(theme).join("index.theme");
            if let Ok(contents) = fs::read_to_string(&index_path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if let Some(value) = line.strip_prefix("Inherits=") {
                        let parents: Vec<String> = value
                            .split(',')
                            .map(|s| s.trim().trim_matches(['\'', '\"']).to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if !parents.is_empty() {
                            return Some(parents);
                        }
                    }
                }
            }
        }
        None
    }

    let icon_dirs = get_icon_dirs();

    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([theme.to_string()]);

    while let Some(theme_name) = queue.pop_front() {
        if !visited.insert(theme_name.clone()) {
            continue;
        }
        chain.push(theme_name.clone());

        if let Some(parents) = parse_inherits(&theme_name, &icon_dirs) {
            for parent in parents {
                queue.push_back(parent);
            }
        }
    }

    if !visited.contains("hicolor") {
        chain.push("hicolor".to_string());
    }

    chain
}

// Resolve icon path from theme (supports svg, png, xpm, scalable, symbolic, etc.)
pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
    if icon_name.is_empty() {
        return None;
    }

    let path = Path::new(icon_name);

    // If already a valid file path
    if path.exists() {
        return Some(path.to_path_buf());
    }

    let config = Config::get();
    let icon_dirs = get_icon_dirs();
    let theme_chain = &config.icon_theme_chain;

    let subdirs = [
        "apps",
        "status",
        "actions",
        "devices",
        "categories",
        "places",
        "mimetypes",
    ];

    let sizes = [
        "scalable", "symbolic", "16x16", "22x22", "24x24", "32x32", "48x48", "64x64", "128x128",
    ];

    let extensions = ["svg", "svgz", "png", "xpm"];

    // NOTE: This is a decent amount of filesystem access every time we get a notification.
    // Maybe we should consider some kind of cache for icon names->icon paths table?
    for theme_name in theme_chain {
        for base_dir in &icon_dirs {
            let base_path = Path::new(base_dir);

            if !base_path.exists() {
                continue;
            }

            for size in &sizes {
                for subdir in &subdirs {
                    let dir_path = base_path.join(theme_name).join(size).join(subdir);

                    if !dir_path.is_dir() {
                        continue;
                    }

                    for ext in &extensions {
                        let full = dir_path.join(format!("{}.{}", icon_name, ext));

                        if full.exists() {
                            return Some(full);
                        }
                    }
                }
            }
        }
    }

    // Last resort: search directly in /usr/share/pixmaps.
    for ext in &extensions {
        let full = Path::new("/usr/share/pixmaps").join(format!("{}.{}", icon_name, ext));
        if full.exists() {
            return Some(full);
        }
    }

    eprintln!("Icon '{}' not found in any theme path.", icon_name);
    None
}

/// Expand ~ to home directory manually (no external crates)
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            let mut p = PathBuf::from(home);
            p.push(path.trim_start_matches("~/"));
            return p.display().to_string();
        }
    }
    path.to_string()
}
