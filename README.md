# GloriousEvolution

Voice dictation for any Windows app. Press a hotkey, speak, and clean, polished text is typed at your cursor.

Everything runs on a single [OpenRouter](https://openrouter.ai) API key:

- **Speech-to-text** via OpenRouter's `/api/v1/audio/transcriptions` (default: `openai/gpt-4o-mini-transcribe`)
- **Polishing** via `/api/v1/chat/completions` (default: `google/gemini-3.1-flash-lite`): removes filler words, repetitions and self-corrections, fixes punctuation and homophones, keeps your meaning

Typical cost is well under $0.01 per minute of dictation. OpenRouter only serves audio requests when the account balance is at least $0.50, so free-tier keys need a small top-up.

## Features

- Global hotkey, either toggle (press to start, press again to finish) or hold-to-talk; `Esc` cancels
- Types directly into the focused app; if there is no text field, a floating preview offers one-click copy
- Personal dictionary for names and jargon
- Choose speech and polish models, language, and microphone
- Local history with search and copy, plus usage stats
- Tray app with launch at login, single instance, and optional sound cues
- If polishing fails, the raw transcript is still delivered, so a dictation is never lost
- 16 kHz mono capture held in memory; no audio is written to disk

## Setup

1. Create a key at <https://openrouter.ai/settings/keys> and add a few dollars of credit.
2. Launch the app, paste the key, click **测试连接**, then **开始使用**.
3. Focus any text field and press `Alt+Q`.

## Development

Requirements: Node 20+, Rust stable (MSVC toolchain), and WebView2 (preinstalled on Windows 10/11).

```bash
npm install
npm run tauri dev      # run in development
npm run tauri build    # produce the NSIS installer in src-tauri/target/release/bundle/nsis
```

Settings live in `%APPDATA%\com.glorious.evolution\settings.json`, and history lives in `history.json` in the same folder.
