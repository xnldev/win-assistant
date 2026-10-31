use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// API key for Gemini or OpenAI
    pub api_key: String,

    /// Optional backup API key (automatically used if primary exceeds quota 429)
    #[serde(default)]
    pub backup_api_key: Option<String>,

    /// Provider: "gemini" (recommended) or "openai"
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Model name: "gemini-flash-latest", "gemini-3.8-flash", "gpt-4o", etc.
    #[serde(default = "default_model")]
    pub model: String,

    /// System instructions for the vision/text LLM
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,

    /// Optional custom URL for OpenAI-compatible proxies (OpenRouter, Groq, Ollama)
    #[serde(default)]
    pub api_url: Option<String>,

    /// Window and visual settings
    #[serde(default)]
    pub window: WindowConfig,

    /// Hotkey configurations
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
}

fn default_provider() -> String {
    "gemini".to_string()
}

fn default_model() -> String {
    "gemini-3.5-flash".to_string()
}

fn default_system_prompt() -> String {
    "You are an expert exam, quiz, and technical problem solver. Carefully inspect the entire screen or text. If there are MULTIPLE questions or tasks, answer EVERY single one sequentially from top to bottom (1, 2, 3...) without skipping any. For each question: 1) State the question number/title, 2) Give the exact correct answer (option letter like A, B, C, D and full option text, or exact value), 3) Provide a super-concise 1-sentence explanation only if needed, 4) For code/commands, provide clean copy-paste ready blocks. Match the language of the questions (Ukrainian/English). No introductory filler or conversational fluff.".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window width in pixels
    #[serde(default = "default_width")]
    pub width: i32,

    /// Window height in pixels
    #[serde(default = "default_height")]
    pub height: i32,

    /// Offset from top of the screen (centered horizontally)
    #[serde(default = "default_top_offset")]
    pub top_offset: i32,

    /// Default opacity percentage (30 to 100)
    #[serde(default = "default_opacity")]
    pub opacity_percent: u8,

    /// Monospace base font size
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// TUI font family
    #[serde(default = "default_font_family")]
    pub font_family: String,

    /// Initial state of mouse follow mode
    #[serde(default = "default_false")]
    pub follow_mouse: bool,
}

fn default_width() -> i32 { 860 }
fn default_height() -> i32 { 480 }
fn default_top_offset() -> i32 { 35 }
fn default_opacity() -> u8 { 94 }
fn default_font_size() -> f32 { 14.0 }
fn default_font_family() -> String { "Consolas".to_string() }
fn default_false() -> bool { false }

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            top_offset: default_top_offset(),
            opacity_percent: default_opacity(),
            font_size: default_font_size(),
            font_family: default_font_family(),
            follow_mouse: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Alt + Q: Silent screen capture & Vision LLM analysis
    #[serde(default = "default_hk_screen")]
    pub capture_screen: String,

    /// Alt + W: Analyze clipboard text
    #[serde(default = "default_hk_clipboard")]
    pub analyze_clipboard: String,

    /// Alt + S: Toggle window visibility (without focus loss)
    #[serde(default = "default_hk_visibility")]
    pub toggle_visibility: String,

    /// Alt + Up: Scroll output up
    #[serde(default = "default_hk_scroll_up")]
    pub scroll_up: String,

    /// Alt + Down: Scroll output down
    #[serde(default = "default_hk_scroll_down")]
    pub scroll_down: String,

    /// Alt + Left: Decrease opacity
    #[serde(default = "default_hk_opacity_dec")]
    pub opacity_decrease: String,

    /// Alt + Right: Increase opacity
    #[serde(default = "default_hk_opacity_inc")]
    pub opacity_increase: String,

    /// Alt + Shift + X: Panic exit
    #[serde(default = "default_hk_panic")]
    pub panic_exit: String,

    /// Alt + F: Toggle mouse follow mode
    #[serde(default = "default_hk_follow")]
    pub toggle_follow: String,

    /// Alt + M: Cycle window presets (Compact / Standard / Wide)
    #[serde(default = "default_hk_presets")]
    pub cycle_presets: String,

    /// Alt + [: Decrease width
    #[serde(default = "default_hk_w_dec")]
    pub width_decrease: String,

    /// Alt + ]: Increase width
    #[serde(default = "default_hk_w_inc")]
    pub width_increase: String,

    /// Alt + -: Decrease height
    #[serde(default = "default_hk_h_dec")]
    pub height_decrease: String,

    /// Alt + =: Increase height
    #[serde(default = "default_hk_h_inc")]
    pub height_increase: String,

    /// Alt + C: Clear chat history
    #[serde(default = "default_hk_clear")]
    pub clear_history: String,
}

fn default_hk_screen() -> String { "Q".to_string() }
fn default_hk_clipboard() -> String { "W".to_string() }
fn default_hk_visibility() -> String { "S".to_string() }
fn default_hk_scroll_up() -> String { "Up".to_string() }
fn default_hk_scroll_down() -> String { "Down".to_string() }
fn default_hk_opacity_dec() -> String { "Left".to_string() }
fn default_hk_opacity_inc() -> String { "Right".to_string() }
fn default_hk_panic() -> String { "X".to_string() }
fn default_hk_follow() -> String { "F".to_string() }
fn default_hk_presets() -> String { "M".to_string() }
fn default_hk_w_dec() -> String { "[".to_string() }
fn default_hk_w_inc() -> String { "]".to_string() }
fn default_hk_h_dec() -> String { "-".to_string() }
fn default_hk_h_inc() -> String { "=".to_string() }
fn default_hk_clear() -> String { "C".to_string() }

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            capture_screen: default_hk_screen(),
            analyze_clipboard: default_hk_clipboard(),
            toggle_visibility: default_hk_visibility(),
            scroll_up: default_hk_scroll_up(),
            scroll_down: default_hk_scroll_down(),
            opacity_decrease: default_hk_opacity_dec(),
            opacity_increase: default_hk_opacity_inc(),
            panic_exit: default_hk_panic(),
            toggle_follow: default_hk_follow(),
            cycle_presets: default_hk_presets(),
            width_decrease: default_hk_w_dec(),
            width_increase: default_hk_w_inc(),
            height_decrease: default_hk_h_dec(),
            height_increase: default_hk_h_inc(),
            clear_history: default_hk_clear(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: "AQ.Ab8RN6L_czzg24ZyEfygIBpnjNEFXBx4CtL54xQ4yTxK-NiKig".to_string(),
            backup_api_key: None,
            provider: default_provider(),
            model: default_model(),
            system_prompt: default_system_prompt(),
            api_url: None,
            window: WindowConfig::default(),
            hotkeys: HotkeyConfig::default(),
        }
    }
}

pub const DEFAULT_CONFIG_TEMPLATE: &str = r##"# Win-Assistant

api_key = "AQ.Ab8RN6L_czzg24ZyEfygIBpnjNEFXBx4CtL54xQ4yTxK-NiKig"
# backup_api_key = "OPTIONAL_SECOND_GEMINI_KEY" # Auto-switches if primary hits 429 quota
provider = "gemini"
model = "gemini-3.5-flash"

system_prompt = "You are an expert exam, quiz, and technical problem solver. Carefully inspect the entire screen. If there are MULTIPLE questions or tasks, answer EVERY single one sequentially (1, 2, 3...) without skipping any. For each question: give the question number, exact correct option letter and text (e.g. 'Answer: B) ...'), and super-concise code or explanation. Match the language of the test. No introductory filler."

[window]
width = 860
height = 500
top_offset = 35
opacity_percent = 92
font_size = 13.5
font_family = "Consolas"
follow_mouse = false

[hotkeys]
capture_screen = "Q"       # Alt + Q: Scan screen
analyze_clipboard = "W"    # Alt + W: Analyze clipboard
toggle_visibility = "S"    # Alt + S: Hide/show HUD
toggle_follow = "F"        # Alt + F: Follow cursor
cycle_presets = "M"        # Alt + M: Cycle window sizes
width_decrease = "["       # Alt + [: Narrower (-60px)
width_increase = "]"       # Alt + ]: Wider (+60px)
height_decrease = "-"      # Alt + -: Shorter (-50px)
height_increase = "="      # Alt + =: Taller (+50px)
clear_history = "C"        # Alt + C: Clear chat history
scroll_up = "Up"           # Alt + Up: Scroll up
scroll_down = "Down"       # Alt + Down: Scroll down
opacity_decrease = "Left"  # Alt + Left: Less opaque
opacity_increase = "Right" # Alt + Right: More opaque
panic_exit = "X"           # Alt + Shift + X: Exit
"##;

impl Config {
    pub fn load_or_create() -> Result<Self, String> {
        // Try multiple candidate paths to find the real configured file
        let mut candidates = Vec::new();

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                candidates.push(parent.join("config.toml"));
            }
        }
        if let Ok(cur_dir) = std::env::current_dir() {
            candidates.push(cur_dir.join("config.toml"));
        }

        // 1. Look for an existing file with a configured key
        for candidate in &candidates {
            if candidate.exists() {
                if let Ok(content) = fs::read_to_string(candidate) {
                    if let Ok(cfg) = toml::from_str::<Config>(&content) {
                        if !cfg.api_key.trim().is_empty() && cfg.api_key != "YOUR_GEMINI_OR_OPENAI_API_KEY" {
                            println!("[CONFIG] Loaded active config from: {:?}", candidate);
                            return Ok(cfg);
                        }
                    }
                }
            }
        }

        // 2. Check any existing candidate
        for candidate in &candidates {
            if candidate.exists() {
                if let Ok(content) = fs::read_to_string(candidate) {
                    if let Ok(cfg) = toml::from_str::<Config>(&content) {
                        return Ok(cfg);
                    }
                }
            }
        }

        // 3. Create default template if not found
        let target_path = candidates.into_iter().next().unwrap_or_else(|| PathBuf::from("config.toml"));
        let _ = fs::write(&target_path, DEFAULT_CONFIG_TEMPLATE);
        println!("[CONFIG] Created default config.toml at: {:?}", target_path);

        Ok(Config::default())
    }

    /// Returns list of valid configured API keys in priority order (primary first, then backup)
    pub fn get_active_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        let primary = self.api_key.trim();
        if !primary.is_empty() && primary != "YOUR_GEMINI_OR_OPENAI_API_KEY" {
            keys.push(primary.to_string());
        }
        if let Some(ref bk) = self.backup_api_key {
            let backup = bk.trim();
            if !backup.is_empty() && backup != "YOUR_BACKUP_KEY_HERE" && !keys.contains(&backup.to_string()) {
                keys.push(backup.to_string());
            }
        }
        keys
    }
}
