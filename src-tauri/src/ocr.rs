//! OCR fallback for capturing text from non-selectable content.
//!
//! When clipboard capture returns empty (e.g. images, PDFs), this module
//! captures a screen region around the cursor and runs Windows.Media.Ocr
//! to extract text.

// =============================================================================
// Public types
// =============================================================================

/// Text captured via OCR near the cursor position.
#[derive(Debug, Clone)]
pub struct CapturedText {
    /// The single word closest to the cursor.
    pub word: String,
    /// The sentence containing that word.
    pub sentence: String,
    /// All text recognized in the captured region.
    pub all_text: String,
}

/// A recognized word with its bounding box (our own type, decoupled from WinRT).
#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub bounds: OcrBounds,
}

/// Bounding rectangle in pixels relative to the capture region.
#[derive(Debug, Clone)]
pub struct OcrBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A line of recognized text containing individual words.
#[derive(Debug, Clone)]
pub struct OcrLine {
    pub text: String,
    pub words: Vec<OcrWord>,
}

// =============================================================================
// Constants
// =============================================================================

/// Width of the screen capture region in physical pixels.
const CAPTURE_WIDTH: i32 = 500;
/// Height of the screen capture region in physical pixels.
const CAPTURE_HEIGHT: i32 = 250;

// =============================================================================
// Public API
// =============================================================================

/// Capture text near the current cursor position via OCR.
///
/// Returns `Ok(None)` when OCR finds no text in the captured region.
/// This is blocking (WinRT RecognizeAsync uses `.get()`), so call from
/// `spawn_blocking` or a background thread.
#[cfg(windows)]
pub fn capture_text_near_cursor() -> Result<Option<CapturedText>, String> {
    let (cursor_x, cursor_y) = get_cursor_position()?;

    // Capture region centered on cursor, clamped to screen bounds.
    let (screen_w, screen_h) = get_screen_dimensions()?;

    let region_x = (cursor_x - CAPTURE_WIDTH / 2).clamp(0, screen_w - CAPTURE_WIDTH);
    let region_y = (cursor_y - CAPTURE_HEIGHT / 2).clamp(0, screen_h - CAPTURE_HEIGHT);

    let bitmap_data = capture_screen_region(region_x, region_y, CAPTURE_WIDTH, CAPTURE_HEIGHT)?;

    let lines = run_ocr(&bitmap_data, CAPTURE_WIDTH, CAPTURE_HEIGHT)?;

    if lines.is_empty() {
        return Ok(None);
    }

    // Cursor position relative to the capture region.
    let rel_x = (cursor_x - region_x) as f64;
    let rel_y = (cursor_y - region_y) as f64;

    Ok(find_word_near_cursor(&lines, rel_x, rel_y))
}

// =============================================================================
// Private helpers — Win32 / WinRT
// =============================================================================

/// Get current cursor position in physical pixels.
#[cfg(windows)]
fn get_cursor_position() -> Result<(i32, i32), String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point).map_err(|e| format!("GetCursorPos failed: {}", e))?;
    }
    Ok((point.x, point.y))
}

/// Get primary screen dimensions in physical pixels.
#[cfg(windows)]
fn get_screen_dimensions() -> Result<(i32, i32), String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    };

    let w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if w == 0 || h == 0 {
        return Err("GetSystemMetrics returned 0 for screen dimensions".into());
    }
    Ok((w, h))
}

/// Capture a screen region as raw BGRA pixel data via BitBlt + GetDIBits.
#[cfg(windows)]
fn capture_screen_region(x: i32, y: i32, width: i32, height: i32) -> Result<Vec<u8>, String> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
        SRCCOPY,
    };

    unsafe {
        // Get the screen device context (None = entire screen).
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err("GetDC failed".into());
        }

        // Create a compatible memory DC and bitmap.
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.is_invalid() {
            ReleaseDC(None, screen_dc);
            return Err("CreateCompatibleDC failed".into());
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return Err("CreateCompatibleBitmap failed".into());
        }

        let old_bitmap = SelectObject(mem_dc, bitmap.into());

        // Copy screen region to memory DC.
        let blt_result = BitBlt(mem_dc, 0, 0, width, height, Some(screen_dc), x, y, SRCCOPY);
        if blt_result.is_err() {
            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
            return Err("BitBlt failed".into());
        }

        // Prepare BITMAPINFO for 32-bit BGRA.
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Top-down bitmap (negative = top-down).
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: (width * height * 4) as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        // Extract pixel data.
        let buffer_size = (width * height * 4) as usize;
        let mut pixels: Vec<u8> = vec![0u8; buffer_size];

        let scan_lines = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup GDI resources.
        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);

        if scan_lines == 0 {
            return Err("GetDIBits failed".into());
        }

        Ok(pixels)
    }
}

/// Run Windows.Media.Ocr on raw BGRA pixel data.
///
/// Uses `.get()` to block on the WinRT async operation.
#[cfg(windows)]
fn run_ocr(bitmap_data: &[u8], width: i32, height: i32) -> Result<Vec<OcrLine>, String> {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    // Create an IBuffer from the raw pixel data via DataWriter.
    let writer = DataWriter::new().map_err(|e| format!("DataWriter::new failed: {}", e))?;
    writer
        .WriteBytes(bitmap_data)
        .map_err(|e| format!("WriteBytes failed: {}", e))?;
    let buffer = writer
        .DetachBuffer()
        .map_err(|e| format!("DetachBuffer failed: {}", e))?;

    // Create a SoftwareBitmap from the BGRA pixel buffer.
    let software_bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width,
        height,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(|e| format!("SoftwareBitmap creation failed: {}", e))?;

    // Create OCR engine from user profile languages.
    let engine = OcrEngine::TryCreateFromUserProfileLanguages()
        .map_err(|e| format!("OcrEngine creation failed: {}", e))?;

    // Run OCR (WinRT async → blocking .get()).
    let result = engine
        .RecognizeAsync(&software_bitmap)
        .map_err(|e| format!("RecognizeAsync failed: {}", e))?
        .get()
        .map_err(|e| format!("OCR recognition failed: {}", e))?;

    // Convert WinRT OcrResult into our own types.
    let winrt_lines = result
        .Lines()
        .map_err(|e| format!("Failed to get OCR lines: {}", e))?;

    let mut lines = Vec::new();
    for i in 0..winrt_lines.Size().unwrap_or(0) {
        let winrt_line = winrt_lines
            .GetAt(i)
            .map_err(|e| format!("Failed to get line {}: {}", i, e))?;

        let line_text = winrt_line
            .Text()
            .map_err(|e| format!("Failed to get line text: {}", e))?
            .to_string();

        let winrt_words = winrt_line
            .Words()
            .map_err(|e| format!("Failed to get words: {}", e))?;

        let mut words = Vec::new();
        for j in 0..winrt_words.Size().unwrap_or(0) {
            let winrt_word = winrt_words
                .GetAt(j)
                .map_err(|e| format!("Failed to get word {}: {}", j, e))?;

            let text = winrt_word
                .Text()
                .map_err(|e| format!("Failed to get word text: {}", e))?
                .to_string();

            let rect = winrt_word
                .BoundingRect()
                .map_err(|e| format!("Failed to get bounding rect: {}", e))?;

            words.push(OcrWord {
                text,
                bounds: OcrBounds {
                    x: rect.X as f64,
                    y: rect.Y as f64,
                    width: rect.Width as f64,
                    height: rect.Height as f64,
                },
            });
        }

        lines.push(OcrLine {
            text: line_text,
            words,
        });
    }

    Ok(lines)
}

// =============================================================================
// Pure helper functions (testable without Win32)
// =============================================================================

/// Find the word nearest to the cursor position within OCR results.
///
/// Uses Euclidean distance from cursor to center of each word's bounding box.
fn find_word_near_cursor(
    lines: &[OcrLine],
    cursor_x: f64,
    cursor_y: f64,
) -> Option<CapturedText> {
    let mut best_word: Option<&OcrWord> = None;
    let mut best_line: Option<&OcrLine> = None;
    let mut best_distance = f64::MAX;

    for line in lines {
        for word in &line.words {
            let center_x = word.bounds.x + word.bounds.width / 2.0;
            let center_y = word.bounds.y + word.bounds.height / 2.0;
            let dx = cursor_x - center_x;
            let dy = cursor_y - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < best_distance {
                best_distance = distance;
                best_word = Some(word);
                best_line = Some(line);
            }
        }
    }

    let word = best_word?;
    let line = best_line?;

    // Find the word's position within the line text to extract sentence.
    let word_pos = line.text.find(&word.text).unwrap_or(0);

    let all_text = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Some(CapturedText {
        word: word.text.clone(),
        sentence: extract_sentence(&line.text, word_pos),
        all_text,
    })
}

/// Extract a single word from text at the given character index.
///
/// Expands outward from `char_index` to find word boundaries (alphanumeric + hyphens).
fn extract_word(text: &str, char_index: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = text.chars().collect();
    let idx = char_index.min(chars.len().saturating_sub(1));

    // If the char at index is not a word char, return empty.
    if !is_word_char(chars[idx]) {
        return String::new();
    }

    // Expand left.
    let mut start = idx;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    // Expand right.
    let mut end = idx;
    while end + 1 < chars.len() && is_word_char(chars[end + 1]) {
        end += 1;
    }

    chars[start..=end].iter().collect()
}

/// Extract the sentence containing the character at `char_index`.
///
/// A sentence boundary is one of: `.` `!` `?` followed by whitespace, or
/// the start/end of the text.
fn extract_sentence(text: &str, char_index: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = text.chars().collect();
    let idx = char_index.min(chars.len().saturating_sub(1));

    // Find sentence start: scan left for sentence-ending punctuation followed by whitespace.
    let mut start = 0;
    for i in (0..idx).rev() {
        if is_sentence_end(chars[i]) {
            // The sentence starts after this punctuation + whitespace.
            let next = i + 1;
            if next < chars.len() && chars[next].is_whitespace() {
                start = next + 1;
            } else {
                start = next;
            }
            break;
        }
    }

    // Find sentence end: scan right for sentence-ending punctuation.
    let mut end = chars.len();
    for i in idx..chars.len() {
        if is_sentence_end(chars[i]) {
            end = i + 1; // Include the punctuation.
            break;
        }
    }

    let result: String = chars[start..end].iter().collect();
    result.trim().to_string()
}

/// Check if a character is part of a "word" (alphanumeric, hyphen, apostrophe, underscore).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '\'' || c == '_'
}

/// Check if a character is a sentence-ending punctuation mark.
fn is_sentence_end(c: char) -> bool {
    c == '.' || c == '!' || c == '?'
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // extract_word tests
    // =========================================================================

    #[test]
    fn extract_word_simple() {
        assert_eq!(extract_word("hello world", 0), "hello");
    }

    #[test]
    fn extract_word_middle() {
        assert_eq!(extract_word("hello world", 7), "world");
    }

    #[test]
    fn extract_word_with_hyphen() {
        assert_eq!(extract_word("well-known fact", 2), "well-known");
    }

    #[test]
    fn extract_word_with_apostrophe() {
        assert_eq!(extract_word("it's fine", 2), "it's");
    }

    #[test]
    fn extract_word_at_boundary() {
        assert_eq!(extract_word("hello world", 5), ""); // space
    }

    #[test]
    fn extract_word_empty_text() {
        assert_eq!(extract_word("", 0), "");
    }

    #[test]
    fn extract_word_index_past_end() {
        assert_eq!(extract_word("hello", 100), "hello");
    }

    #[test]
    fn extract_word_single_char() {
        assert_eq!(extract_word("a", 0), "a");
    }

    #[test]
    fn extract_word_underscore() {
        assert_eq!(extract_word("my_var = 1", 1), "my_var");
    }

    // =========================================================================
    // extract_sentence tests
    // =========================================================================

    #[test]
    fn extract_sentence_single() {
        assert_eq!(extract_sentence("Hello world.", 3), "Hello world.");
    }

    #[test]
    fn extract_sentence_multiple() {
        let text = "First sentence. Second sentence. Third sentence.";
        assert_eq!(extract_sentence(text, 17), "Second sentence.");
    }

    #[test]
    fn extract_sentence_exclamation() {
        let text = "Wow! That is great.";
        assert_eq!(extract_sentence(text, 0), "Wow!");
    }

    #[test]
    fn extract_sentence_question() {
        let text = "How are you? I am fine.";
        assert_eq!(extract_sentence(text, 2), "How are you?");
    }

    #[test]
    fn extract_sentence_no_punctuation() {
        assert_eq!(extract_sentence("no punctuation here", 5), "no punctuation here");
    }

    #[test]
    fn extract_sentence_empty() {
        assert_eq!(extract_sentence("", 0), "");
    }

    #[test]
    fn extract_sentence_index_past_end() {
        assert_eq!(extract_sentence("Hello.", 100), "Hello.");
    }

    #[test]
    fn extract_sentence_at_last_sentence() {
        let text = "One. Two. Three.";
        assert_eq!(extract_sentence(text, 12), "Three.");
    }

    // =========================================================================
    // find_word_near_cursor tests
    // =========================================================================

    fn make_line(text: &str, words: Vec<(&str, f64, f64, f64, f64)>) -> OcrLine {
        OcrLine {
            text: text.to_string(),
            words: words
                .into_iter()
                .map(|(t, x, y, w, h)| OcrWord {
                    text: t.to_string(),
                    bounds: OcrBounds {
                        x,
                        y,
                        width: w,
                        height: h,
                    },
                })
                .collect(),
        }
    }

    #[test]
    fn find_word_near_cursor_picks_closest() {
        let lines = vec![make_line(
            "hello world",
            vec![
                ("hello", 10.0, 10.0, 50.0, 20.0), // center: (35, 20)
                ("world", 70.0, 10.0, 50.0, 20.0),  // center: (95, 20)
            ],
        )];

        let result = find_word_near_cursor(&lines, 40.0, 20.0).unwrap();
        assert_eq!(result.word, "hello");
    }

    #[test]
    fn find_word_near_cursor_picks_second_word() {
        let lines = vec![make_line(
            "hello world",
            vec![
                ("hello", 10.0, 10.0, 50.0, 20.0),
                ("world", 70.0, 10.0, 50.0, 20.0),
            ],
        )];

        let result = find_word_near_cursor(&lines, 90.0, 20.0).unwrap();
        assert_eq!(result.word, "world");
    }

    #[test]
    fn find_word_near_cursor_returns_sentence() {
        let lines = vec![
            make_line(
                "Hello world.",
                vec![
                    ("Hello", 10.0, 10.0, 50.0, 20.0),
                    ("world.", 70.0, 10.0, 50.0, 20.0),
                ],
            ),
            make_line(
                "Goodbye now.",
                vec![
                    ("Goodbye", 10.0, 40.0, 70.0, 20.0),
                    ("now.", 90.0, 40.0, 30.0, 20.0),
                ],
            ),
        ];

        let result = find_word_near_cursor(&lines, 50.0, 50.0).unwrap();
        assert_eq!(result.word, "Goodbye");
        assert_eq!(result.sentence, "Goodbye now.");
    }

    #[test]
    fn find_word_near_cursor_all_text_joins_lines() {
        let lines = vec![
            make_line(
                "Line one.",
                vec![("Line", 0.0, 0.0, 40.0, 20.0), ("one.", 50.0, 0.0, 30.0, 20.0)],
            ),
            make_line(
                "Line two.",
                vec![("Line", 0.0, 30.0, 40.0, 20.0), ("two.", 50.0, 30.0, 30.0, 20.0)],
            ),
        ];

        let result = find_word_near_cursor(&lines, 20.0, 10.0).unwrap();
        assert_eq!(result.all_text, "Line one. Line two.");
    }

    #[test]
    fn find_word_near_cursor_empty_lines() {
        let lines: Vec<OcrLine> = vec![];
        assert!(find_word_near_cursor(&lines, 50.0, 50.0).is_none());
    }

    #[test]
    fn find_word_near_cursor_no_words_in_line() {
        let lines = vec![OcrLine {
            text: "".to_string(),
            words: vec![],
        }];
        assert!(find_word_near_cursor(&lines, 50.0, 50.0).is_none());
    }

    #[test]
    fn find_word_near_cursor_multiline_closest() {
        let lines = vec![
            make_line(
                "top line",
                vec![
                    ("top", 100.0, 10.0, 30.0, 20.0),
                    ("line", 140.0, 10.0, 40.0, 20.0),
                ],
            ),
            make_line(
                "bottom line",
                vec![
                    ("bottom", 100.0, 100.0, 60.0, 20.0),
                    ("line", 170.0, 100.0, 40.0, 20.0),
                ],
            ),
        ];

        // Cursor near the bottom-left area.
        let result = find_word_near_cursor(&lines, 120.0, 95.0).unwrap();
        assert_eq!(result.word, "bottom");
    }

    // =========================================================================
    // is_word_char / is_sentence_end tests
    // =========================================================================

    #[test]
    fn is_word_char_alpha() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
    }

    #[test]
    fn is_word_char_digit() {
        assert!(is_word_char('5'));
    }

    #[test]
    fn is_word_char_special() {
        assert!(is_word_char('-'));
        assert!(is_word_char('\''));
        assert!(is_word_char('_'));
    }

    #[test]
    fn is_word_char_space() {
        assert!(!is_word_char(' '));
    }

    #[test]
    fn is_sentence_end_punctuation() {
        assert!(is_sentence_end('.'));
        assert!(is_sentence_end('!'));
        assert!(is_sentence_end('?'));
    }

    #[test]
    fn is_sentence_end_not_comma() {
        assert!(!is_sentence_end(','));
        assert!(!is_sentence_end(' '));
        assert!(!is_sentence_end('a'));
    }

    // =========================================================================
    // Integration test (requires running on Windows with a display)
    // =========================================================================

    #[test]
    #[ignore]
    #[cfg(windows)]
    fn integration_capture_text_near_cursor() {
        // Run with: cargo test integration_capture_text_near_cursor -- --ignored
        // Position your cursor over some text on screen before running.
        let result = capture_text_near_cursor();
        match result {
            Ok(Some(captured)) => {
                println!("Word: {}", captured.word);
                println!("Sentence: {}", captured.sentence);
                println!("All text: {}", captured.all_text);
                assert!(!captured.word.is_empty());
            }
            Ok(None) => {
                println!("No text detected near cursor (this is OK if cursor is over a blank area)");
            }
            Err(e) => {
                panic!("OCR capture failed: {}", e);
            }
        }
    }
}
