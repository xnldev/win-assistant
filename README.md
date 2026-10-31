# Win-Assistant

> **Invisible AI HUD for Windows.** Solves tests, and code directly on screen without stealing focus. Completely invisible to screen capture and screen sharing (OBS, Discord, Zoom, Meet).

---

## Quick Start

1. **[Download the latest `win-assistant.zip`](https://github.com/xnldev/win-assistant/releases/latest)** from Releases.
2. Unpack the archive (`win-assistant.exe` and `config.toml`).
3. Open `config.toml` and paste your free [Google Gemini API Key](https://aistudio.google.com/app/apikey):
   ```toml
   api_key = "YOUR_GEMINI_KEY"
   ```
4. Run `win-assistant.exe` and press **`Alt + Q`** on any test or problem.

---

## General Hotkeys

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
