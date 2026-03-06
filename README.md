# Quill for Windows

System-wide AI tech dictionary. Select any term, press **Ctrl+Alt+Q**, get an instant explanation at your level.

## Features

- **Tech Dictionary** — Explain technical terms with 6 depth levels (ELI5 to Pro)
- **Improve** — Fix grammar, spelling, and suggest richer vocabulary
- **Translate** — Auto-detect and translate between languages
- **Drill-down** — Click `[[linked terms]]` to explore related concepts
- **OCR Fallback** — Capture text from images and non-selectable content
- **Non-activating Panel** — Floating panel doesn't steal focus from your app

## Requirements

- Windows 10 version 1903+ (WebView2 + Windows.Media.Ocr)
- API key for [Gemini](https://aistudio.google.com/apikey) or [Claude](https://console.anthropic.com/)

## Installation

Download the latest `.msi` installer from [Releases](../../releases).

## Build from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/) (or Node.js 18+)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with "Desktop development with C++"
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually pre-installed on Windows 10/11)

### Steps

```bash
git clone https://github.com/c3nx/quill-windows.git
cd quill-windows
bun install
bun run tauri build
```

The built installer will be at `src-tauri/target/release/bundle/`.

## Usage

1. Configure your API key in **Settings** (system tray > Settings)
2. Select any text in any application
3. Press **Ctrl+Alt+Q**
4. The floating panel appears with an explanation
5. Press **ESC** to dismiss, or click **Apply** to replace text (Improve mode)

### Modes

| Mode | Shortcut | Description |
|------|----------|-------------|
| Tech Dictionary | Default | Explains technical terms at your chosen level |
| Improve | Via tray menu | Fixes grammar and suggests vocabulary |
| Translate | Via tray menu | Auto-detects language and translates |

### Explanation Levels (Tech Dictionary)

| Level | Description |
|-------|-------------|
| ELI5 | Simple, everyday language |
| ELI15 | Clear with some technical terms |
| Pro | Precise, trade-offs, design patterns |
| Samples | 2-3 practical code examples |
| Resources | Quick bullet points and related concepts |
| Alternatives | Compare 3-5 competing tools/libraries |

## Architecture

Tauri v2 (Rust backend + React frontend) with hexagonal architecture.

```
src-tauri/src/
  ai/           # AI prompts, response parser, Gemini + Claude clients
  models/       # Domain models (AnalysisMode, ExplanationLevel, etc.)
  clipboard.rs  # Clipboard text capture via Ctrl+C simulation
  ocr.rs        # Windows.Media.Ocr fallback
  panel.rs      # Win32 non-activating floating panel
  tray.rs       # System tray with mode picker
  keyring_manager.rs  # Windows Credential Manager
  app_state.rs  # Shared application state
  lib.rs        # Main flow: hotkey -> capture -> analyze -> panel

src/
  components/   # React UI (FloatingPanel, Settings, etc.)
  hooks/        # useAnalysis, useDrillDown
  lib/          # TypeScript types
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| App shell | Tauri v2 |
| Backend | Rust |
| Frontend | React 19 + TypeScript + Tailwind CSS |
| AI | Gemini API, Claude API |
| Build | Cargo + Vite + Bun |

## License

MIT
