// Ayuz - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

//! COSMIC panel configuration writers.
//!
//! `cosmic-panel` reads its configuration from
//! `~/.config/cosmic/com.system76.CosmicPanel.<Name>/v1/<key>` and watches those
//! files via inotify, so changes apply immediately without a panel restart.

use directories::BaseDirs;
use rust_i18n::t;
use std::path::PathBuf;

const AUTOHIDE_ON: &str =
    "Some((wait_time: 1000, transition_time: 200, handle_size: 4, unhide_delay: 200))";
const AUTOHIDE_OFF: &str = "None";
const OPACITY_TRANSPARENT: &str = "0.0";
const OPACITY_OPAQUE: &str = "1.0";

/// Enables or disables auto-hide on every COSMIC panel found in the user's config.
pub(crate) async fn set_autohide_all(enabled: bool) -> Result<(), String> {
    let value = if enabled { AUTOHIDE_ON } else { AUTOHIDE_OFF };
    write_to_all_panels("autohide", value).await
}

/// Sets every COSMIC panel's opacity to transparent (`0.0`) or fully opaque (`1.0`).
pub(crate) async fn set_opacity_all(transparent: bool) -> Result<(), String> {
    let value = if transparent {
        OPACITY_TRANSPARENT
    } else {
        OPACITY_OPAQUE
    };
    write_to_all_panels("opacity", value).await
}

async fn write_to_all_panels(key: &'static str, value: &'static str) -> Result<(), String> {
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let Some(dirs) = panel_config_dirs()? else {
            return Ok(());
        };
        for dir in dirs {
            let v1 = dir.join("v1");
            std::fs::create_dir_all(&v1).map_err(|e| e.to_string())?;
            std::fs::write(v1.join(key), value).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| t!("error_spawn_blocking", error = e.to_string()).to_string())?
}

fn panel_config_dirs() -> Result<Option<Vec<PathBuf>>, String> {
    let Some(base) = BaseDirs::new() else {
        return Ok(None);
    };
    let cosmic_dir = base.config_dir().join("cosmic");
    if !cosmic_dir.is_dir() {
        return Ok(None);
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&cosmic_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("com.system76.CosmicPanel.")
            && !name.starts_with("com.system76.CosmicPanelButton")
        {
            out.push(path);
        }
    }
    Ok(Some(out))
}
