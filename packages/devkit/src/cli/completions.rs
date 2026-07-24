use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::Cli;

/// Supported shell types for completion generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
}

impl ShellType {
    /// Parse a shell name string into a `ShellType`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            _ => None,
        }
    }

    /// Return the corresponding `clap_complete::Shell` value.
    pub fn to_clap_shell(self) -> Shell {
        match self {
            Self::Bash => Shell::Bash,
            Self::Zsh => Shell::Zsh,
            Self::Fish => Shell::Fish,
        }
    }
}

impl std::fmt::Display for ShellType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Zsh => write!(f, "zsh"),
            Self::Fish => write!(f, "fish"),
        }
    }
}

/// Arguments for the `completions` subcommand.
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    pub shell: ShellType,
}

impl Default for CompletionsArgs {
    fn default() -> Self {
        Self {
            shell: ShellType::Bash,
        }
    }
}

impl CompletionsArgs {
    /// Generate and print the shell completion script to stdout.
    pub fn run(&self) {
        let mut cmd = Cli::command();
        generate(self.shell.to_clap_shell(), &mut cmd, "devkit", &mut io::stdout());
    }

    /// Generate the completion script into a `String` (useful for testing).
    pub fn generate_to_string(&self) -> String {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(self.shell.to_clap_shell(), &mut cmd, "devkit", &mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_type_parse_bash() {
        assert_eq!(ShellType::parse("bash"), Some(ShellType::Bash));
    }

    #[test]
    fn shell_type_parse_zsh() {
        assert_eq!(ShellType::parse("zsh"), Some(ShellType::Zsh));
    }

    #[test]
    fn shell_type_parse_fish() {
        assert_eq!(ShellType::parse("fish"), Some(ShellType::Fish));
    }

    #[test]
    fn shell_type_parse_unknown_returns_none() {
        assert_eq!(ShellType::parse("powershell"), None);
    }

    #[test]
    fn shell_type_parse_case_insensitive() {
        assert_eq!(ShellType::parse("ZSH"), Some(ShellType::Zsh));
        assert_eq!(ShellType::parse("BASH"), Some(ShellType::Bash));
        assert_eq!(ShellType::parse("Fish"), Some(ShellType::Fish));
    }

    #[test]
    fn completions_generate_bash_is_nonempty() {
        let args = CompletionsArgs {
            shell: ShellType::Bash,
        };
        let script = args.generate_to_string();
        assert!(!script.is_empty());
    }

    #[test]
    fn completions_generate_zsh_is_nonempty() {
        let args = CompletionsArgs {
            shell: ShellType::Zsh,
        };
        let script = args.generate_to_string();
        assert!(!script.is_empty());
    }

    #[test]
    fn completions_generate_fish_is_nonempty() {
        let args = CompletionsArgs {
            shell: ShellType::Fish,
        };
        let script = args.generate_to_string();
        assert!(!script.is_empty());
    }

    #[test]
    fn completions_default_shell_is_bash() {
        let args = CompletionsArgs::default();
        assert_eq!(args.shell, ShellType::Bash);
    }
}
