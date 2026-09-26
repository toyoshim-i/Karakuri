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
    /// Open-source autonomous coding agent (`cline`).
    Cline,
    /// OpenAI Codex CLI (`codex`).
    Codex,
    /// GitHub Copilot CLI (`copilot`).
    Copilot,
    /// DeepSeek CLI (`deepseek`).
    DeepSeek,
    /// xAI Grok CLI (`grok`).
    Grok,
    /// Nous Hermes AI agent CLI (`hermes`).
    Hermes,
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
    /// OpenCode AI coding assistant CLI (`opencode`).
    OpenCode,
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
        CliPreset::Cline,
        CliPreset::Codex,
        CliPreset::Copilot,
        CliPreset::DeepSeek,
        CliPreset::Grok,
        CliPreset::Hermes,
        CliPreset::Kimi,
        CliPreset::Mimo,
        CliPreset::MiniMax,
        CliPreset::Mistral,
        CliPreset::Muse,
        CliPreset::Ollama,
        CliPreset::OpenCode,
        CliPreset::Pi,
        CliPreset::Qwen,
    ];

    /// Command binary name searched on PATH.
    pub const fn command(self) -> &'static str {
        match self {
            Self::Agy => "agy",
            Self::Aider => "aider",
            Self::Claude => "claude",
            Self::Cline => "cline",
            Self::Codex => "codex",
            Self::Copilot => "copilot",
            Self::DeepSeek => "deepseek",
            Self::Grok => "grok",
            Self::Hermes => "hermes",
            Self::Kimi => "kimi",
            Self::Mimo => "mimo",
            Self::MiniMax => "minimax",
            Self::Mistral => "mistral",
            Self::Muse => "muse",
            Self::Ollama => "ollama",
            Self::OpenCode => "opencode",
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

/// Resolves the file path of `cmd` if found on PATH or as a direct executable file.
pub fn resolve_executable(cmd: &str) -> Option<std::path::PathBuf> {
    if cmd.is_empty() {
        return None;
    }

    // Direct path specified with slashes
    if cmd.contains('/') || (cfg!(windows) && cmd.contains('\\')) {
        let p = std::path::PathBuf::from(cmd);
        if is_executable_file(&p) {
            return Some(p);
        }
        return None;
    }

    let path_var = std::env::var_os("PATH")?;

    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            for ext in &["exe", "cmd", "bat"] {
                let with_ext = dir.join(format!("{cmd}.{ext}"));
                if is_executable_file(&with_ext) {
                    return Some(with_ext);
                }
            }
        }
    }
    None
}

/// Checks whether `cmd` exists and is executable on the system PATH.
pub fn is_executable_on_path(cmd: &str) -> bool {
    resolve_executable(cmd).is_some()
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
