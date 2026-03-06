//! Clipboard-based text capture and paste injection via Win32 SendInput.
//!
//! Uses Ctrl+C / Ctrl+V simulation to capture selected text and paste
//! replacement text in the foreground application.

#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_C, VK_CONTROL,
    VK_LWIN, VK_MENU, VK_SHIFT, VK_V,
};

use tauri_plugin_clipboard_manager::ClipboardExt;

/// Build the INPUT array for a Ctrl+<key> press-release sequence.
///
/// Returns 4 INPUT events: Ctrl down, key down, key up, Ctrl up.
/// This is a pure helper extracted for testability.
#[cfg(windows)]
fn build_ctrl_key_inputs(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> [INPUT; 4] {
    [
        // Press Ctrl
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    ..Default::default()
                },
            },
        },
        // Press key
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    ..Default::default()
                },
            },
        },
        // Release key
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        // Release Ctrl
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ]
}

/// Simulate Ctrl+C keystroke via Win32 SendInput.
// Integration: requires interactive testing
#[cfg(windows)]
fn simulate_ctrl_c() {
    let inputs = build_ctrl_key_inputs(VK_C);
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Simulate Ctrl+V keystroke via Win32 SendInput.
// Integration: requires interactive testing
#[cfg(windows)]
fn simulate_ctrl_v() {
    let inputs = build_ctrl_key_inputs(VK_V);
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Release all modifier keys (Ctrl, Alt, Shift, Win).
///
/// Must be called before Ctrl+C simulation because the hotkey
/// Ctrl+Alt+Q leaves Alt held down. Without releasing it, SendInput
/// produces Ctrl+Alt+C instead of Ctrl+C.
#[cfg(windows)]
fn release_modifiers() {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_CONTROL,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_MENU,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_SHIFT,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_LWIN,
                    dwFlags: KEYEVENTF_KEYUP,
                    ..Default::default()
                },
            },
        },
    ];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Delay duration after simulating keystrokes (ms).
const KEYSTROKE_DELAY_MS: u64 = 100;

/// Capture the currently selected text via clipboard.
///
/// 1. Release modifier keys (Ctrl+Alt from hotkey still held)
/// 2. Save current clipboard contents
/// 3. Simulate Ctrl+C via SendInput
/// 4. Sleep for target app to process copy
/// 5. Read clipboard
/// 6. Restore original clipboard
/// 7. Return captured text (empty string if clipboard unchanged)
#[cfg(windows)]
pub async fn capture_selected_text(app: &tauri::AppHandle) -> Result<String, String> {
    // 1. Release modifier keys left over from hotkey (Ctrl+Alt+Q)
    release_modifiers();

    // Brief delay for modifier release to take effect
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(30));
    })
    .await
    .map_err(|e| format!("Sleep task failed: {}", e))?;

    // 2. Save current clipboard contents
    let original = app
        .clipboard()
        .read_text()
        .unwrap_or_default();

    // 3. Simulate Ctrl+C
    simulate_ctrl_c();

    // 4. Wait for the target app to process the copy
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(KEYSTROKE_DELAY_MS));
    })
    .await
    .map_err(|e| format!("Sleep task failed: {}", e))?;

    // 5. Read clipboard
    let captured = app
        .clipboard()
        .read_text()
        .map_err(|e| format!("Failed to read clipboard: {}", e))?;

    // 6. Restore original clipboard
    if !original.is_empty() {
        let _ = app.clipboard().write_text(&original);
    }

    // 7. Return captured text (empty if unchanged)
    if captured == original {
        Ok(String::new())
    } else {
        Ok(captured)
    }
}

/// Paste text by writing to clipboard and simulating Ctrl+V.
///
/// 1. Write text to clipboard
/// 2. Sleep 50ms
/// 3. Simulate Ctrl+V via SendInput
#[cfg(windows)]
pub async fn paste_text(app: &tauri::AppHandle, text: &str) -> Result<(), String> {
    // 1. Write text to clipboard
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("Failed to write clipboard: {}", e))?;

    // 2. Wait briefly before simulating paste
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(KEYSTROKE_DELAY_MS));
    })
    .await
    .map_err(|e| format!("Sleep task failed: {}", e))?;

    // 3. Simulate Ctrl+V
    simulate_ctrl_v();

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT_KEYBOARD, KEYEVENTF_KEYUP, VK_C, VK_CONTROL, VK_V,
    };

    // =========================================================================
    // Unit tests: pure helper functions
    // =========================================================================

    #[test]
    fn build_ctrl_c_inputs_has_four_events() {
        let inputs = build_ctrl_key_inputs(VK_C);
        assert_eq!(inputs.len(), 4);
    }

    #[test]
    fn build_ctrl_v_inputs_has_four_events() {
        let inputs = build_ctrl_key_inputs(VK_V);
        assert_eq!(inputs.len(), 4);
    }

    #[test]
    fn all_inputs_are_keyboard_type() {
        let inputs = build_ctrl_key_inputs(VK_C);
        for input in &inputs {
            assert_eq!(input.r#type, INPUT_KEYBOARD);
        }
    }

    #[test]
    fn first_event_is_ctrl_down() {
        let inputs = build_ctrl_key_inputs(VK_C);
        let ki = unsafe { inputs[0].Anonymous.ki };
        assert_eq!(ki.wVk, VK_CONTROL);
        assert!(!ki.dwFlags.contains(KEYEVENTF_KEYUP));
    }

    #[test]
    fn second_event_is_key_down() {
        let inputs = build_ctrl_key_inputs(VK_C);
        let ki = unsafe { inputs[1].Anonymous.ki };
        assert_eq!(ki.wVk, VK_C);
        assert!(!ki.dwFlags.contains(KEYEVENTF_KEYUP));
    }

    #[test]
    fn third_event_is_key_up() {
        let inputs = build_ctrl_key_inputs(VK_C);
        let ki = unsafe { inputs[2].Anonymous.ki };
        assert_eq!(ki.wVk, VK_C);
        assert!(ki.dwFlags.contains(KEYEVENTF_KEYUP));
    }

    #[test]
    fn fourth_event_is_ctrl_up() {
        let inputs = build_ctrl_key_inputs(VK_C);
        let ki = unsafe { inputs[3].Anonymous.ki };
        assert_eq!(ki.wVk, VK_CONTROL);
        assert!(ki.dwFlags.contains(KEYEVENTF_KEYUP));
    }

    #[test]
    fn ctrl_v_uses_correct_virtual_key() {
        let inputs = build_ctrl_key_inputs(VK_V);
        let ki_down = unsafe { inputs[1].Anonymous.ki };
        let ki_up = unsafe { inputs[2].Anonymous.ki };
        assert_eq!(ki_down.wVk, VK_V);
        assert_eq!(ki_up.wVk, VK_V);
    }

    #[test]
    fn keystroke_delay_is_100ms() {
        assert_eq!(KEYSTROKE_DELAY_MS, 100);
    }

    // =========================================================================
    // Integration tests: require interactive UI environment.
    // Run manually with: cargo test -- --ignored
    // =========================================================================

    #[test]
    #[ignore]
    fn simulate_ctrl_c_does_not_panic() {
        // Integration: requires interactive testing
        // Open a text editor, select some text, then run this test.
        simulate_ctrl_c();
    }

    #[test]
    #[ignore]
    fn simulate_ctrl_v_does_not_panic() {
        // Integration: requires interactive testing
        simulate_ctrl_v();
    }
}
