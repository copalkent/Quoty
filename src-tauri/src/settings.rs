//! UI preferences. Contains no credentials or session data.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SavedPosition {
    pub x: f64,
    pub y: f64,
}

impl SavedPosition {
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub theme: Theme,
    pub always_on_top: bool,
    pub start_with_windows: bool,
    pub expanded: bool,
    pub locked: bool,
    pub width: Option<f64>,
    pub position: Option<SavedPosition>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            always_on_top: false,
            start_with_windows: true,
            expanded: true,
            locked: false,
            width: Some(540.0),
            position: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferencesPatch {
    pub theme: Option<Theme>,
    pub always_on_top: Option<bool>,
    pub start_with_windows: Option<bool>,
    pub expanded: Option<bool>,
    pub locked: Option<bool>,
    pub width: Option<f64>,
    pub position: Option<SavedPosition>,
}

impl Preferences {
    pub fn apply(&mut self, patch: PreferencesPatch) {
        if let Some(theme) = patch.theme {
            self.theme = theme;
        }
        if let Some(value) = patch.always_on_top {
            self.always_on_top = value;
        }
        if let Some(value) = patch.start_with_windows {
            self.start_with_windows = value;
        }
        if let Some(value) = patch.expanded {
            self.expanded = value;
        }
        if let Some(value) = patch.locked {
            self.locked = value;
        }
        if let Some(w) = patch.width {
            if w.is_finite() && w >= 400.0 && w <= 900.0 {
                self.width = Some(w);
            }
        }
        if let Some(position) = patch.position {
            if position.is_finite() {
                self.position = Some(position);
            }
        }
    }

    fn sanitize(&mut self) {
        if self.position.is_some_and(|p| !p.is_finite()) {
            self.position = None;
        }
    }
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Preferences {
    let Some(path) = settings_path(app) else {
        return Preferences::default();
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return Preferences::default();
    };
    match serde_json::from_str::<Preferences>(&contents) {
        Ok(mut prefs) => {
            prefs.sanitize();
            prefs.expanded = true;
            prefs
        }
        Err(_) => Preferences::default(),
    }
}

pub fn save(app: &AppHandle, prefs: &Preferences) -> Result<(), String> {
    let Some(path) = settings_path(app) else {
        return Err("Could not resolve the settings directory.".into());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let serialized = serde_json::to_string_pretty(prefs).map_err(|error| error.to_string())?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serialized).map_err(|error| error.to_string())?;
    fs::rename(&temp, &path).map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_preferences_are_private_and_minimal() {
        let prefs = Preferences::default();
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(!json.contains("csrf"));
        assert!(!json.contains("token"));
        assert!(!prefs.always_on_top);
        assert!(prefs.start_with_windows);
        assert!(prefs.expanded);
        assert_eq!(prefs.theme, Theme::System);
    }

    #[test]
    fn patch_only_touches_given_fields() {
        let mut prefs = Preferences::default();
        prefs.apply(PreferencesPatch {
            theme: Some(Theme::Dark),
            ..Default::default()
        });
        assert_eq!(prefs.theme, Theme::Dark);
        assert!(!prefs.always_on_top);
    }

    #[test]
    fn non_finite_positions_are_ignored() {
        let mut prefs = Preferences::default();
        prefs.apply(PreferencesPatch {
            position: Some(SavedPosition {
                x: f64::INFINITY,
                y: 10.0,
            }),
            ..Default::default()
        });
        assert!(prefs.position.is_none());
    }

    #[test]
    fn unknown_settings_fields_are_rejected() {
        let parsed = serde_json::from_str::<Preferences>(
            r#"{"theme":"dark","alwaysOnTop":false,"startWithWindows":true,"expanded":true,"position":null}"#,
        )
        .unwrap();
        assert_eq!(parsed.theme, Theme::Dark);
        assert!(!parsed.always_on_top);

        let patch = serde_json::from_str::<PreferencesPatch>(r#"{"theme":"system","csrf":"secret"}"#);
        assert!(patch.is_err());
    }

    #[test]
    fn malformed_theme_falls_back_to_default() {
        assert!(serde_json::from_str::<Preferences>(r#"{"theme":"violet"}"#).is_err());
    }
}
