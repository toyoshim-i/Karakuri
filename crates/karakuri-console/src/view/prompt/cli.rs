//! CLI presets, executable detection, and session selection for the Prompt bay.

/// Built-in agent CLI presets supported by the Prompt bay, ordered alphabetically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CliPreset {
    /// Google Antigravity CLI (`agy`).
    Agy,
    /// Open-source AI pair programmer (`aider`).
    Aider,
    /// Anthropic Claude Code CLI (`claude`).
    Claude,
    /// OpenAI Codex CLI (`codex`).
    Codex,
    /// GitHub Copilot CLI (`copilot`).
    Copilot,
    /// DeepSeek CLI (`deepseek`).
    DeepSeek,
    /// xAI Grok CLI (`grok`).
    Grok,
    /// Moonshot Kimi CLI (`kimi`).
    Kimi,
    /// Xiaomi MiMo CLI (`mimo`).
    Mimo,
    /// MiniMax CLI (`minimax`).
    MiniMax,
    /// Mistral AI CLI (`mistral`).
    Mistral,
    /// Muse CLI (`muse`).
    Muse,
    /// Ollama local LLM runner (`ollama`).
    Ollama,
    /// Pi CLI (`pi`).
    Pi,
    /// Alibaba Qwen CLI (`qwen`).
    Qwen,
}

impl CliPreset {
    /// All preset agent CLIs, in strict alphabetical order.
    pub const ALL: &'static [CliPreset] = &[
        CliPreset::Agy,
        CliPreset::Aider,
        CliPreset::Claude,
        CliPreset::Codex,
        CliPreset::Copilot,
        CliPreset::DeepSeek,
        CliPreset::Grok,
        CliPreset::Kimi,
        CliPreset::Mimo,
        CliPreset::MiniMax,
        CliPreset::Mistral,
        CliPreset::Muse,
        CliPreset::Ollama,
        CliPreset::Pi,
        CliPreset::Qwen,
    ];

    /// Command binary name searched on PATH.
    pub const fn command(self) -> &'static str {
        match self {
            Self::Agy => "agy",
            Self::Aider => "aider",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Copilot => "copilot",
            Self::DeepSeek => "deepseek",
            Self::Grok => "grok",
            Self::Kimi => "kimi",
            Self::Mimo => "mimo",
            Self::MiniMax => "minimax",
            Self::Mistral => "mistral",
            Self::Muse => "muse",
            Self::Ollama => "ollama",
            Self::Pi => "pi",
            Self::Qwen => "qwen",
        }
    }

    /// Display name shown in dropdown and header pill.
    pub const fn display_name(self) -> &'static str {
        self.command()
    }

    /// Checks if this preset's binary is available on the system PATH.
    pub fn is_available(self) -> bool {
        is_executable_on_path(self.command())
    }
}

/// Active selection for the Prompt bay.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CliSelection {
    /// No agent CLI session selected.
    #[default]
    Unselected,
    /// One of the standard presets selected.
    Preset(CliPreset),
    /// Custom user-specified command.
    Custom(String),
}

impl CliSelection {
    /// Whether no CLI is currently selected.
    pub fn is_unselected(&self) -> bool {
        matches!(self, Self::Unselected)
    }

    /// Label to display on the header selector pill.
    pub fn pill_label(&self) -> String {
        match self {
            Self::Unselected => "prompt ▾".to_owned(),
            Self::Preset(preset) => format!("{} ▾", preset.display_name()),
            Self::Custom(cmd) if cmd.trim().is_empty() => "custom... ▾".to_owned(),
            Self::Custom(cmd) => format!("{} ▾", cmd.trim()),
        }
    }
}

/// Checks whether `cmd` exists and is executable on the system PATH.
pub fn is_executable_on_path(cmd: &str) -> bool {
    if cmd.is_empty() {
        return false;
    }

    // Direct path specified with slashes
    if cmd.contains('/') || (cfg!(windows) && cmd.contains('\\')) {
        let p = std::path::Path::new(cmd);
        return is_executable_file(p);
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };

    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if is_executable_file(&candidate) {
            return true;
        }
        #[cfg(windows)]
        {
            for ext in &["exe", "cmd", "bat"] {
                let with_ext = dir.join(format!("{cmd}.{ext}"));
                if is_executable_file(&with_ext) {
                    return true;
                }
            }
        }
    }
    false
}

fn is_executable_file(path: &std::path::Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
