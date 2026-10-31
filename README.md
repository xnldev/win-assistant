# Win-Assistant

A clean, lightweight, zero-dependency Windows AI HUD overlay built with native Win32 & Direct2D. 

Completely invisible to screen capture & screen sharing (OBS, Zoom, Discord, Google Meet, Teams). Never steals focus from your active apps or games.

---

## Features

- **OBS / Screen-Share Invisibility**: Hardware-level window capture protection (`WDA_EXCLUDEFROMCAPTURE`).
- **Zero Focus Stealing**: Click-through, non-activating window (`WS_EX_NOACTIVATE | WS_EX_TRANSPARENT`).
- **Task Queue System**: Press `Alt + Q` repeatedly — all requests are queued in memory and executed smoothly without drops.
- **Micro-Animations & Inertia Scrolling**: Smooth continuous scrolling (`Alt + ▲ / ▼`), fluid opacity fading (`Alt + ◄ / ►`), and cyberpunk status laser glow.
- **Failover / Backup Key**: Seamless automatic fallback to a backup API key if quota limits are reached.
- **Native Direct2D**: Zero Electron, zero webviews, zero heavy runtimes — instant startup with ~40MB RAM usage.

---

## Global Hotkeys

| Shortcut | Action | Description |
| :--- | :--- | :--- |
| **`Alt + Q`** | **Scan Screen** | Captures active screen in RAM and queries AI with task queue |
| **`Alt + W`** | **Scan Clipboard** | Solves copied text / code from clipboard |
| **`Alt + C`** | **Clear History** | Clears chat stream and resets view |
| **`Alt + F`** | **Follow Cursor** | Glides smoothly following mouse position |
| **`Alt + M`** | **Cycle Sizes** | Compact (680×380), Standard (860×500), Wide (1120×680) |
| **`Alt + [` / `Alt + ]`** | **Adjust Width** | Shrink / expand width (-60px / +60px) |
| **`Alt + -` / `Alt + =`** | **Adjust Height** | Shrink / expand height (-50px / +50px) |
| **`Alt + S`** | **Hide / Show** | Toggle HUD visibility instantly without losing focus |
| **`Alt + ▲` / `Alt + ▼`** | **Smooth Scroll** | Hold for fluid inertia scroll or tap for line scroll |
| **`Alt + ◄` / `Alt + ►`** | **Opacity** | Smoothly adjust transparency (30% to 98%) |
| **`Alt + Shift + X`** | **Exit** | Close the assistant cleanly |

---

## Quick Setup (For Users)

No need to install Rust or any build tools!

1. Download `win-assistant.exe` and `config.example.toml`.
2. Rename `config.example.toml` to `config.toml` (place it next to `win-assistant.exe`).
3. Get a free Gemini API key from [Google AI Studio](https://aistudio.google.com/app/apikey).
4. Put your key into `config.toml`:
   ```toml
   [api]
   api_key = "YOUR_GEMINI_KEY"
   backup_api_key = "" # optional backup key
   model = "gemini-flash-latest"
   ```
5. Run `win-assistant.exe`. Press **`Alt + Q`** anytime!

---

## Build from Source (For Developers)

Requires Windows 10/11 and Rust 1.80+:

```powershell
cargo build --release
```

Produces a single, self-contained `win-assistant.exe` in `target/release/`.
