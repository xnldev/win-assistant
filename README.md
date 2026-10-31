# Win-Assistant

> **Invisible AI HUD for Windows.** Solves quizzes, tests, and code directly on screen without stealing focus. Completely invisible to screen capture & screen sharing (OBS, Discord, Zoom, Meet, Teams).

---

## ⚡ Quick Start (No Coding or Rust Needed)

1. **[Download the latest `win-assistant.zip`](https://github.com/xnldev/win-assistant/releases/latest)** from Releases.
2. Unpack the archive (`win-assistant.exe` and `config.toml`).
3. Open `config.toml` and paste your free [Google Gemini API Key](https://aistudio.google.com/app/apikey):
   ```toml
   api_key = "YOUR_GEMINI_KEY"
   ```
4. Run `win-assistant.exe` and press **`Alt + Q`** on any test or problem!

---

## ⌨️ Essential Hotkeys

| Hotkey | Action | What it does |
| :--- | :--- | :--- |
| **`Alt + Q`** | **Scan Screen** | Takes a screenshot to RAM and gets instant AI answer |
| **`Alt + W`** | **Clipboard** | Solves copied text or code |
| **`Alt + C`** | **Clear** | Clears chat stream and queue |
| **`Alt + F`** | **Follow Mouse** | Floats smoothly next to cursor |
| **`Alt + S`** | **Hide / Show** | Toggles overlay visibility |
| **`Alt + ▲ / ▼`** | **Scroll** | Smooth scrolling through answers |
| **`Alt + ◄ / ►`** | **Opacity** | Adjust transparency (30% – 98%) |
| **`Alt + M`** | **Resize** | Cycles window size presets |
| **`Alt + Shift + X`** | **Exit** | Closes the app |

---

## 🔒 Stealth Guarantee

* **100% Screen-Share Proof**: Uses native Windows `WDA_EXCLUDEFROMCAPTURE`. Discord, OBS, Zoom, or Teams see only your desktop wallpaper or apps behind it.
* **No Focus Stealing**: Built with `WS_EX_NOACTIVATE | WS_EX_TRANSPARENT`. Active windows and games never lose keyboard/mouse focus.
* **Ultra-lightweight**: Pure Win32 + Direct2D in Rust. ~40 MB RAM, instant startup, zero Electron/Node overhead.
