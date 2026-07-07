# Telegram HTML → Markdown Cleaner

A fast, privacy-focused desktop app that converts Telegram chat HTML exports into clean, anonymized Markdown files.

## Features

- **Drag & Drop** or browse to add HTML files from Telegram's "Export chat history" feature
- **Anonymization options** (each can be toggled independently):
  - Participant names → `[Participant N]`
  - Phone numbers → `[PHONE]`
  - Email addresses → `[EMAIL]`
  - Web links & URLs → `[LINK]`
  - Credit / debit card numbers → `[CARD]`
  - Physical addresses → `[ADDRESS]`
  - API keys, bot tokens, access tokens → `[TOKEN]`
- **Real-time log** with per-file message counts
- **Fast** — processes thousands of messages per second using background threads
- **Portable & Standalone** — compiles to a single native binary (no installer required) for Windows, macOS, and Linux
- **100% local** — no data is ever sent anywhere

## Usage

1. Export your Telegram chat: **Menu → Settings → Advanced → Export Telegram Data → Only HTML**
2. Open the application binary (e.g., `tg-anonymizer.exe` on Windows)
3. Drag the exported `messages*.html` files (or the whole folder) onto the app
4. Configure which data to anonymize in the **Settings** tab
5. Click **Start Cleaning** and choose where to save the output `.md` file

## Building from source

Requires [Rust](https://rustup.rs/) (stable, 1.75+) and C/C++ build tools appropriate for your platform.

```sh
git clone https://github.com/meoteq/tg-anonymizer
cd tg-anonymizer
cargo build --release
```

The compiled binary will be located at:
* Windows: `target/release/tg-anonymizer.exe`
* macOS/Linux: `target/release/tg-anonymizer`

## Privacy

All processing happens **100% locally** — no data is ever sent anywhere. Your chat history never leaves your machine.

## License

GPL-3.0
