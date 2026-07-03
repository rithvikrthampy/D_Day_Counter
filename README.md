# D-daycounter

A sci‑fi themed **desktop countdown widget** for Windows. Set a target date and
D‑daycounter shows a live, always‑available countdown in a frameless neon panel —
perfect for launches, deadlines, exams, trips, or any "D‑day".

Built with [Tauri 2](https://tauri.app/) (Rust + web frontend), so it's tiny,
fast, and runs as a native Windows app.

## Screenshots

| Widget | Settings |
| --- | --- |
| ![The countdown widget](screenshots/widget.png) | ![The settings panel](screenshots/settings.png) |

## Features

- ⏳ **Live countdown** to any target date & time — days, hours, minutes, seconds.
- 📊 **"Time tactical index"** progress bar showing how much time has elapsed.
- 🎨 **Four neon themes** — Cyber Cyan, Solar Orange, Bio Green, Neon Pink.
- 🏷️ **Custom event name** for whatever you're counting down to.
- 📌 **Always‑on‑top** toggle to keep the widget above other windows.
- 🚀 **Autostart on boot** so the widget is ready every time you log in.
- 🪟 **Frameless & draggable** — grab the header to move it anywhere.
- 💾 **Settings persist** locally between launches.

## Download & install

Grab the latest installer from the [**Releases**](https://github.com/rithvikrthampy/D_Day_Counter/releases/latest) page:

- **`D-daycounter_x64-setup.exe`** — recommended Windows installer (NSIS).
- **`D-daycounter_x64_en-US.msi`** — alternative Windows Installer package.

> Because the app isn't code‑signed yet, Windows SmartScreen may show a
> "Windows protected your PC" prompt on first run. Click **More info →
> Run anyway** to proceed.

After installing, launch **D-daycounter**, open settings (⚙), set your event and
target date, and optionally enable **Autostart on boot**.

## Build from source

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org/) 18+
- Windows build tools for Tauri — see the
  [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)
  (Microsoft C++ Build Tools + WebView2, which ships with Windows 11).

### Run in development

```bash
npm install
npm run tauri dev
```

### Build the installers

```bash
npm run tauri build
```

The installers are written to:

```
src-tauri/target/release/bundle/nsis/D-daycounter_<version>_x64-setup.exe
src-tauri/target/release/bundle/msi/D-daycounter_<version>_x64_en-US.msi
```

## Tech stack

- **[Tauri 2](https://tauri.app/)** — native shell & Rust backend
- **Rust** — autostart via the Windows registry (`HKCU\...\Run`)
- **Vanilla HTML / CSS / JavaScript** — no frontend framework

## License

Released under the [MIT License](LICENSE).
