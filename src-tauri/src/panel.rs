//! Win32 panel window management for non-activating floating panel.
//!
//! Sets WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW on the panel window so it
//! never steals focus from the user's application. Positions the panel
//! near the cursor with DPI-aware monitor clamping.

#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use tauri::Manager;

/// Panel width in pixels (must match tauri.conf.json).
const PANEL_WIDTH: i32 = 420;
/// Panel height in pixels (must match tauri.conf.json).
const PANEL_HEIGHT: i32 = 500;
/// Cursor offset in pixels (panel appears slightly below-right of cursor).
const CURSOR_OFFSET: i32 = 15;

/// Set WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW on the panel window.
///
/// Called once during app setup so the panel never steals focus or
/// appears in the taskbar/Alt-Tab list.
#[cfg(windows)]
pub fn setup_panel_window(app: &tauri::AppHandle) -> Result<(), String> {
    let panel = app
        .get_webview_window("panel")
        .ok_or("Panel window not found")?;
    let hwnd = panel.hwnd().map_err(|e| e.to_string())?;

    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ex_style | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize,
        );
    }

    Ok(())
}

/// Show the panel near the current cursor position.
///
/// Uses SWP_NOACTIVATE to avoid stealing focus. The position is clamped
/// to the monitor work area so the panel never goes off-screen.
#[cfg(windows)]
pub fn show_panel_near_cursor(app: &tauri::AppHandle) -> Result<(), String> {
    let panel = app
        .get_webview_window("panel")
        .ok_or("Panel window not found")?;
    let hwnd = panel.hwnd().map_err(|e| e.to_string())?;

    // Get cursor position in physical pixels.
    let mut cursor = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut cursor).map_err(|e| format!("GetCursorPos failed: {}", e))?;
    }

    // Get the work area of the monitor containing the cursor.
    let work_area = get_monitor_work_area(cursor)?;

    // Calculate panel position, clamped to monitor bounds.
    let (x, y) = clamp_to_work_area(
        cursor.x,
        cursor.y,
        work_area,
    );

    // Show and position the panel without activating it.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            PANEL_WIDTH,
            PANEL_HEIGHT,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|e| format!("SetWindowPos failed: {}", e))?;
    }

    Ok(())
}

/// Hide the panel window.
#[cfg(windows)]
pub fn hide_panel(app: &tauri::AppHandle) -> Result<(), String> {
    let panel = app
        .get_webview_window("panel")
        .ok_or("Panel window not found")?;
    panel.hide().map_err(|e| e.to_string())?;
    Ok(())
}

// =============================================================================
// Work area rectangle (decoupled from Win32 for testability)
// =============================================================================

/// A rectangle representing a monitor work area (excludes taskbar).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Get the work area of the monitor containing the given point.
#[cfg(windows)]
fn get_monitor_work_area(point: POINT) -> Result<WorkArea, String> {
    unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return Err("GetMonitorInfoW failed".into());
        }

        Ok(WorkArea {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        })
    }
}

/// Calculate panel position clamped to the monitor work area.
///
/// Default: panel appears at (cursor_x + CURSOR_OFFSET, cursor_y + CURSOR_OFFSET).
/// If that would push the panel off the right edge, flip to cursor_x - PANEL_WIDTH - CURSOR_OFFSET.
/// If that would push the panel off the bottom edge, flip to cursor_y - PANEL_HEIGHT - CURSOR_OFFSET.
fn clamp_to_work_area(cursor_x: i32, cursor_y: i32, area: WorkArea) -> (i32, i32) {
    // Try below-right of cursor first.
    let mut x = cursor_x + CURSOR_OFFSET;
    let mut y = cursor_y + CURSOR_OFFSET;

    // If panel extends past the right edge, flip to left of cursor.
    if x + PANEL_WIDTH > area.right {
        x = cursor_x - PANEL_WIDTH - CURSOR_OFFSET;
    }

    // If panel extends past the bottom edge, flip to above cursor.
    if y + PANEL_HEIGHT > area.bottom {
        y = cursor_y - PANEL_HEIGHT - CURSOR_OFFSET;
    }

    // Final clamp: ensure panel stays within work area even after flipping.
    x = x.clamp(area.left, (area.right - PANEL_WIDTH).max(area.left));
    y = y.clamp(area.top, (area.bottom - PANEL_HEIGHT).max(area.top));

    (x, y)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Constants tests
    // =========================================================================

    #[test]
    fn panel_dimensions_match_config() {
        assert_eq!(PANEL_WIDTH, 420);
        assert_eq!(PANEL_HEIGHT, 500);
    }

    #[test]
    fn cursor_offset_is_positive() {
        assert!(CURSOR_OFFSET > 0);
    }

    // =========================================================================
    // clamp_to_work_area tests (pure function, no Win32 needed)
    // =========================================================================

    fn standard_area() -> WorkArea {
        WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040, // 1080 minus 40px taskbar
        }
    }

    #[test]
    fn clamp_normal_position() {
        // Cursor in the center of the screen — panel goes below-right.
        let (x, y) = clamp_to_work_area(500, 300, standard_area());
        assert_eq!(x, 500 + CURSOR_OFFSET);
        assert_eq!(y, 300 + CURSOR_OFFSET);
    }

    #[test]
    fn clamp_flips_left_when_near_right_edge() {
        // Cursor near the right edge — panel should flip to left of cursor.
        let (x, _y) = clamp_to_work_area(1800, 300, standard_area());
        assert_eq!(x, 1800 - PANEL_WIDTH - CURSOR_OFFSET);
    }

    #[test]
    fn clamp_flips_up_when_near_bottom_edge() {
        // Cursor near the bottom edge — panel should flip above cursor.
        let (_x, y) = clamp_to_work_area(500, 900, standard_area());
        assert_eq!(y, 900 - PANEL_HEIGHT - CURSOR_OFFSET);
    }

    #[test]
    fn clamp_flips_both_at_bottom_right_corner() {
        // Cursor at the bottom-right corner.
        let (x, y) = clamp_to_work_area(1800, 900, standard_area());
        assert_eq!(x, 1800 - PANEL_WIDTH - CURSOR_OFFSET);
        assert_eq!(y, 900 - PANEL_HEIGHT - CURSOR_OFFSET);
    }

    #[test]
    fn clamp_stays_within_area_at_top_left() {
        // Cursor at top-left origin.
        let (x, y) = clamp_to_work_area(0, 0, standard_area());
        assert_eq!(x, CURSOR_OFFSET);
        assert_eq!(y, CURSOR_OFFSET);
    }

    #[test]
    fn clamp_handles_multi_monitor_offset() {
        // Second monitor with non-zero origin.
        let area = WorkArea {
            left: 1920,
            top: 0,
            right: 3840,
            bottom: 1040,
        };
        let (x, y) = clamp_to_work_area(2500, 300, area);
        assert_eq!(x, 2500 + CURSOR_OFFSET);
        assert_eq!(y, 300 + CURSOR_OFFSET);
    }

    #[test]
    fn clamp_handles_negative_monitor_coordinates() {
        // Monitor to the left of primary (negative x).
        let area = WorkArea {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1040,
        };
        let (x, y) = clamp_to_work_area(-500, 300, area);
        assert_eq!(x, -500 + CURSOR_OFFSET);
        assert_eq!(y, 300 + CURSOR_OFFSET);
    }

    #[test]
    fn clamp_handles_small_work_area() {
        // Work area smaller than panel — clamps to top-left.
        let area = WorkArea {
            left: 0,
            top: 0,
            right: 200,
            bottom: 200,
        };
        let (x, y) = clamp_to_work_area(100, 100, area);
        // Panel can't fit, so clamped to area.left and area.top.
        assert_eq!(x, 0);
        assert_eq!(y, 0);
    }

    #[test]
    fn clamp_exact_fit_at_edge() {
        // Cursor positioned so panel exactly fits at the right edge.
        let cursor_x = 1920 - PANEL_WIDTH - CURSOR_OFFSET;
        let (x, _y) = clamp_to_work_area(cursor_x, 300, standard_area());
        assert_eq!(x, cursor_x + CURSOR_OFFSET);
        // Verify panel right edge is exactly at work area right.
        assert_eq!(x + PANEL_WIDTH, 1920);
    }

    // =========================================================================
    // WorkArea struct tests
    // =========================================================================

    #[test]
    fn work_area_equality() {
        let a = WorkArea { left: 0, top: 0, right: 1920, bottom: 1080 };
        let b = WorkArea { left: 0, top: 0, right: 1920, bottom: 1080 };
        assert_eq!(a, b);
    }

    #[test]
    fn work_area_clone() {
        let a = WorkArea { left: 100, top: 200, right: 1920, bottom: 1080 };
        let b = a;
        assert_eq!(a, b);
    }

    // =========================================================================
    // Integration tests: require interactive Windows environment.
    // Run manually with: cargo test -- --ignored
    // =========================================================================

    #[test]
    #[ignore]
    #[cfg(windows)]
    fn integration_get_monitor_work_area() {
        let point = POINT { x: 100, y: 100 };
        let area = get_monitor_work_area(point).expect("Should get work area");
        // Work area should have non-zero dimensions.
        assert!(area.right > area.left);
        assert!(area.bottom > area.top);
    }
}
