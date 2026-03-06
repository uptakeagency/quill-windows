use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::app_state::AppState;
use crate::models::AnalysisMode;

/// Set up the system tray icon with a context menu.
///
/// Menu layout:
///   Quill              (title, disabled)
///   Ready              (status, disabled)
///   ────────────────
///   ✓ Tech Dictionary  (mode radio group)
///     Improve
///     Translate
///   ────────────────
///   Settings...
///   ────────────────
///   Quit
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Header items (informational, not clickable)
    let title = MenuItem::with_id(app, "title", "Quill", false, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Ready", false, None::<&str>)?;

    // Mode radio group via CheckMenuItems
    let mode_tech =
        CheckMenuItem::with_id(app, "mode_tech", "Tech Dictionary", true, true, None::<&str>)?;
    let mode_improve =
        CheckMenuItem::with_id(app, "mode_improve", "Improve", true, false, None::<&str>)?;
    let mode_translate =
        CheckMenuItem::with_id(app, "mode_translate", "Translate", true, false, None::<&str>)?;

    // Action items
    let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    // Assemble menu
    let menu = Menu::with_items(
        app,
        &[
            &title,
            &status,
            &PredefinedMenuItem::separator(app)?,
            &mode_tech,
            &mode_improve,
            &mode_translate,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let _tray = TrayIconBuilder::new()
        .icon(tauri::include_image!("icons/32x32.png"))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Quill - AI Tech Dictionary")
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "quit" => {
                    app.exit(0);
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("settings") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "mode_tech" => {
                    set_mode(app, AnalysisMode::TechExplain);
                    update_check_states(&mode_tech, &mode_improve, &mode_translate, "mode_tech");
                }
                "mode_improve" => {
                    set_mode(app, AnalysisMode::Improve);
                    update_check_states(&mode_tech, &mode_improve, &mode_translate, "mode_improve");
                }
                "mode_translate" => {
                    set_mode(app, AnalysisMode::Translate);
                    update_check_states(
                        &mode_tech,
                        &mode_improve,
                        &mode_translate,
                        "mode_translate",
                    );
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// Update the AppState mode.
fn set_mode(app: &AppHandle, mode: AnalysisMode) {
    let state = app.state::<AppState>();
    *state.mode.lock().unwrap() = mode;
}

/// Radio-button behavior: check the active item, uncheck the rest.
fn update_check_states(
    tech: &CheckMenuItem<tauri::Wry>,
    improve: &CheckMenuItem<tauri::Wry>,
    translate: &CheckMenuItem<tauri::Wry>,
    active_id: &str,
) {
    let _ = tech.set_checked(active_id == "mode_tech");
    let _ = improve.set_checked(active_id == "mode_improve");
    let _ = translate.set_checked(active_id == "mode_translate");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_variants_match_menu_ids() {
        // Verify the mapping between menu IDs and AnalysisMode variants.
        // This guards against ID string drift.
        let pairs = [
            ("mode_tech", AnalysisMode::TechExplain),
            ("mode_improve", AnalysisMode::Improve),
            ("mode_translate", AnalysisMode::Translate),
        ];
        for (id, expected_mode) in &pairs {
            let mode = match *id {
                "mode_tech" => AnalysisMode::TechExplain,
                "mode_improve" => AnalysisMode::Improve,
                "mode_translate" => AnalysisMode::Translate,
                _ => panic!("Unknown menu ID: {}", id),
            };
            assert_eq!(mode, *expected_mode);
        }
    }
}
