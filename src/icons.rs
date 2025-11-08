use std::path::{Path, PathBuf};
use std::env;
use crate::config::Config;

/// Resolve icon path from theme (supports svg, png, xpm, scalable, symbolic, etc.)
pub fn resolve_icon_path(icon_name: &str) -> Option<PathBuf> {
    let path = Path::new(icon_name);

    // If already a valid file path
    if path.exists() {
        return Some(path.to_path_buf());
    }

    // Get theme from config or fallback
    let theme = Config::get()
        .icon_theme
        .clone()
        .unwrap_or_else(|| "Adwaita".to_string());


    // Collect all possible icon base directories
    let mut icon_dirs = vec![
        "/usr/share/icons".to_string(),
        "/usr/local/share/icons".to_string(),
        expand_tilde("~/.icons"),
        expand_tilde("~/.local/share/icons"),
        "/usr/share/pixmaps".to_string(),
        "/var/lib/flatpak/exports/share/icons".to_string(),
    ];

    // Add any extra paths from XDG_DATA_DIRS
    if let Ok(xdg_data_dirs) = env::var("XDG_DATA_DIRS") {
        for p in xdg_data_dirs.split(':') {
            let p = format!("{}/icons", p.trim_end_matches('/'));
            icon_dirs.push(p);
        }
    }


    let subdirs = vec![
        "apps", "status", "actions", "devices", "categories", "places", "mimetypes",
    ];

    let sizes = vec![
        "scalable", "symbolic", "16x16", "22x22", "24x24", "32x32", "48x48", "64x64", "128x128",
    ];

    let extensions = vec!["svg", "svgz", "png", "xpm"];

    for base_dir in &icon_dirs {
        let base_path = Path::new(base_dir);

        // skip base dirs that don't exists
        if !base_path.exists() {
            continue;
        }
        for size in &sizes {
            for subdir in &subdirs {
                let dir_path = base_path
                    .join(&theme)
                    .join(size)
                    .join(subdir);

                // skip nonexistent subdirectories to save time
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

    // NOTE: fallback to hicolor theme (optional if you to make an option in config)
    // PERF: skip dirs thats does not exists
    //
    // for base_dir in &icon_dirs {
    //     let base_path = Path::new(base_dir);
    //     if !base_path.exists() {
    //         continue;
    //     }
    //
    //     for size in &sizes {
    //         for subdir in &subdirs {
    //             let dir_path = base_path.join("hicolor").join(size).join(subdir);
    //             if !dir_path.is_dir() {
    //                 continue;
    //             }
    //
    //             for ext in &extensions {
    //                 let full = dir_path.join(format!("{}.{}", icon_name, ext));
    //
    //                 if full.exists() {
    //                     return Some(full);
    //                 }
    //             }
    //         }
    //     }
    // }

    // Last resort: search directly in /usr/share/pixmaps
    // for ext in &extensions {
    //     let full = Path::new("/usr/share/pixmaps").join(format!("{}.{}", icon_name, ext));
    //     if full.exists() {
    //         return Some(full);
    //     }
    // }

    println!("Icon '{}' not found in any theme path.", icon_name);
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
