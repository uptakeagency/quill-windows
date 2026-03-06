use tauri::{Emitter, Manager};

mod models;
mod ai;
mod clipboard;
mod keyring_manager;
mod ocr;
mod panel;
mod app_state;
mod tray;

// =============================================================================
// Tauri commands -- frontend -> backend communication
// =============================================================================

/// Analyze text using the configured AI provider.
///
/// Called from the frontend to run AI analysis on demand (e.g. for drill-down
/// or level switching without going through the hotkey flow).
#[tauri::command]
async fn analyze(
    app: tauri::AppHandle,
    text: String,
    mode: String,
    level: Option<String>,
) -> Result<serde_json::Value, String> {
    let state = app.state::<app_state::AppState>();

    let parsed_mode: models::AnalysisMode =
        serde_json::from_str(&format!("\"{}\"", mode)).map_err(|e| e.to_string())?;

    let parsed_level: Option<models::ExplanationLevel> = match level {
        Some(ref l) => Some(
            serde_json::from_str(&format!("\"{}\"", l)).map_err(|e| e.to_string())?,
        ),
        None => Some(*state.level.lock().unwrap()),
    };

    let tone = state.tone.lock().unwrap().clone();
    let native_lang = state.native_language.lock().unwrap().clone();
    let target_lang = state.target_language.lock().unwrap().clone();
    let provider = state.ai_provider.lock().unwrap().clone();

    let result = call_ai(
        &app,
        &provider,
        &text,
        parsed_mode,
        tone.as_ref(),
        None,
        &native_lang,
        &target_lang,
        parsed_level,
    )
    .await?;

    serde_json::to_value(&result).map_err(|e| e.to_string())
}

/// Apply text replacement in the source application.
///
/// 1. Hide panel
/// 2. Wait briefly for focus to return
/// 3. Paste text via clipboard + Ctrl+V
#[tauri::command]
async fn apply_text(app: tauri::AppHandle, text: String) -> Result<(), String> {
    panel::hide_panel(&app)?;
    clipboard::paste_text(&app, &text).await
}

/// Change the current analysis mode.
#[tauri::command]
fn change_mode(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    let state = app.state::<app_state::AppState>();
    let parsed: models::AnalysisMode =
        serde_json::from_str(&format!("\"{}\"", mode)).map_err(|e| e.to_string())?;
    *state.mode.lock().unwrap() = parsed;
    Ok(())
}

/// Change the current explanation level.
#[tauri::command]
fn change_level(app: tauri::AppHandle, level: String) -> Result<(), String> {
    let state = app.state::<app_state::AppState>();
    let parsed: models::ExplanationLevel =
        serde_json::from_str(&format!("\"{}\"", level)).map_err(|e| e.to_string())?;
    *state.level.lock().unwrap() = parsed;
    Ok(())
}

/// Return current settings as JSON for the settings UI.
#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<app_state::AppState>();
    Ok(serde_json::json!({
        "mode": *state.mode.lock().unwrap(),
        "level": *state.level.lock().unwrap(),
        "tone": *state.tone.lock().unwrap(),
        "nativeLanguage": *state.native_language.lock().unwrap(),
        "targetLanguage": *state.target_language.lock().unwrap(),
        "aiProvider": *state.ai_provider.lock().unwrap(),
        "geminiModel": *state.gemini_model.lock().unwrap(),
        "claudeModel": *state.claude_model.lock().unwrap(),
    }))
}

/// Update AppState from a settings JSON object.
#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: serde_json::Value) -> Result<(), String> {
    let state = app.state::<app_state::AppState>();

    if let Some(mode) = settings.get("mode").and_then(|v| v.as_str()) {
        if let Ok(parsed) = serde_json::from_str::<models::AnalysisMode>(&format!("\"{}\"", mode))
        {
            *state.mode.lock().unwrap() = parsed;
        }
    }
    if let Some(level) = settings.get("level").and_then(|v| v.as_str()) {
        if let Ok(parsed) =
            serde_json::from_str::<models::ExplanationLevel>(&format!("\"{}\"", level))
        {
            *state.level.lock().unwrap() = parsed;
        }
    }
    if let Some(tone) = settings.get("tone") {
        if tone.is_null() {
            *state.tone.lock().unwrap() = None;
        } else if let Some(tone_str) = tone.as_str() {
            if let Ok(parsed) =
                serde_json::from_str::<models::analysis::ToneStyle>(&format!("\"{}\"", tone_str))
            {
                *state.tone.lock().unwrap() = Some(parsed);
            }
        }
    }
    if let Some(lang) = settings.get("nativeLanguage").and_then(|v| v.as_str()) {
        *state.native_language.lock().unwrap() = lang.to_string();
    }
    if let Some(lang) = settings.get("targetLanguage").and_then(|v| v.as_str()) {
        *state.target_language.lock().unwrap() = lang.to_string();
    }
    if let Some(provider) = settings.get("aiProvider").and_then(|v| v.as_str()) {
        *state.ai_provider.lock().unwrap() = provider.to_string();
    }
    if let Some(model) = settings.get("geminiModel").and_then(|v| v.as_str()) {
        *state.gemini_model.lock().unwrap() = model.to_string();
    }
    if let Some(model) = settings.get("claudeModel").and_then(|v| v.as_str()) {
        *state.claude_model.lock().unwrap() = model.to_string();
    }

    Ok(())
}

/// Hide the floating panel (called from frontend e.g. on Escape key).
#[tauri::command]
fn hide_panel_cmd(app: tauri::AppHandle) -> Result<(), String> {
    panel::hide_panel(&app)
}

// =============================================================================
// AI dispatch helper
// =============================================================================

/// Call the configured AI provider (Gemini or Claude).
async fn call_ai(
    app: &tauri::AppHandle,
    provider: &str,
    text: &str,
    mode: models::AnalysisMode,
    tone: Option<&models::analysis::ToneStyle>,
    context: Option<&str>,
    native_language: &str,
    target_language: &str,
    level: Option<models::ExplanationLevel>,
) -> Result<models::AnalysisResult, String> {
    let state = app.state::<app_state::AppState>();

    match provider {
        "claude" => {
            let api_key = keyring_manager::get_api_key(keyring_manager::CLAUDE_KEY)
                .ok_or("Claude API key not configured")?;
            let model = state.claude_model.lock().unwrap().clone();
            ai::claude::analyze(
                &api_key,
                &model,
                text,
                mode,
                tone,
                context,
                Some(native_language),
                Some(target_language),
                level,
            )
            .await
        }
        _ => {
            let api_key = keyring_manager::get_api_key(keyring_manager::GEMINI_KEY)
                .ok_or("Gemini API key not configured")?;
            let model = state.gemini_model.lock().unwrap().clone();
            ai::gemini::analyze(
                &api_key,
                &model,
                text,
                mode,
                tone,
                context,
                Some(native_language),
                Some(target_language),
                level,
            )
            .await
        }
    }
}

// =============================================================================
// Hotkey handler
// =============================================================================

/// Main hotkey handler -- triggered by Ctrl+Alt+Q.
///
/// Flow: capture text -> detect language -> show panel -> call AI -> emit result.
async fn handle_hotkey(app: tauri::AppHandle) -> Result<(), String> {
    // 1. Signal frontend that analysis is starting
    app.emit("analyzing", serde_json::json!({"status": true}))
        .map_err(|e| e.to_string())?;

    // 2. Capture selected text via clipboard (Ctrl+C simulation)
    let clipboard_text = clipboard::capture_selected_text(&app).await?;

    // 3. If clipboard empty, try OCR fallback
    let (text, context) = if clipboard_text.trim().is_empty() {
        match ocr::capture_text_near_cursor() {
            Ok(Some(captured)) => (captured.word.clone(), Some(captured.sentence)),
            Ok(None) => {
                app.emit("analyzing", serde_json::json!({"status": false}))
                    .map_err(|e| e.to_string())?;
                return Err("No text captured".to_string());
            }
            Err(e) => {
                app.emit("analyzing", serde_json::json!({"status": false}))
                    .map_err(|e| e.to_string())?;
                return Err(format!("OCR failed: {}", e));
            }
        }
    } else {
        (clipboard_text, None)
    };

    if text.trim().is_empty() {
        app.emit("analyzing", serde_json::json!({"status": false}))
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 4. Read current state
    let state = app.state::<app_state::AppState>();
    let mode = *state.mode.lock().unwrap();
    let level = *state.level.lock().unwrap();
    let tone = state.tone.lock().unwrap().clone();
    let native_lang = state.native_language.lock().unwrap().clone();
    let target_lang = state.target_language.lock().unwrap().clone();
    let provider = state.ai_provider.lock().unwrap().clone();

    // 5. Emit text-captured event so the frontend can display the term immediately
    app.emit(
        "text-captured",
        serde_json::json!({
            "text": text,
            "mode": mode,
            "context": context,
        }),
    )
    .map_err(|e| e.to_string())?;

    // 6. Show panel near cursor
    panel::show_panel_near_cursor(&app)?;

    // 7. Call AI
    let result = call_ai(
        &app,
        &provider,
        &text,
        mode,
        tone.as_ref(),
        context.as_deref(),
        &native_lang,
        &target_lang,
        Some(level),
    )
    .await;

    // 8. Emit result or error
    match result {
        Ok(analysis) => {
            app.emit(
                "analysis-result",
                serde_json::json!({"result": analysis}),
            )
            .map_err(|e| e.to_string())?;
        }
        Err(e) => {
            app.emit("analysis-error", serde_json::json!({"error": e}))
                .map_err(|e| e.to_string())?;
        }
    }

    // 9. Signal analysis complete
    app.emit("analyzing", serde_json::json!({"status": false}))
        .map_err(|e| e.to_string())?;

    Ok(())
}

// =============================================================================
// App entry point
// =============================================================================

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut("ctrl+alt+q")
                .expect("failed to register Ctrl+Alt+Q shortcut")
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state == ShortcutState::Pressed {
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = handle_hotkey(app_handle).await {
                                log::error!("Hotkey handler error: {}", e);
                            }
                        });
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(app_state::AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();

            // Apply Win32 window styles (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW)
            panel::setup_panel_window(&handle)?;

            // System tray with mode picker, settings access, and quit
            tray::setup_tray(&handle)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            analyze,
            apply_text,
            change_mode,
            change_level,
            get_settings,
            save_settings,
            hide_panel_cmd,
            keyring_manager::get_gemini_key,
            keyring_manager::save_gemini_key,
            keyring_manager::delete_gemini_key,
            keyring_manager::get_claude_key,
            keyring_manager::save_claude_key,
            keyring_manager::delete_claude_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Quill");
}
