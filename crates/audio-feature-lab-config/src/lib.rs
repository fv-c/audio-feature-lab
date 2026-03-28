use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Minimal,
    Default,
    Research,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Default => "default",
            Self::Research => "research",
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "minimal" => Ok(Self::Minimal),
            "default" => Ok(Self::Default),
            "research" => Ok(Self::Research),
            _ => Err(ConfigError::UnknownProfile(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabConfig {
    pub profile: Profile,
}

impl LabConfig {
    pub fn from_path(path: &Path) -> Result<Self, ConfigError> {
        let source = fs::read_to_string(path).map_err(|error| ConfigError::Io {
            path: path.to_path_buf(),
            error,
        })?;

        Self::from_str(&source)
    }

    pub fn from_str(source: &str) -> Result<Self, ConfigError> {
        let mut profile = None;

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let Some((key, raw_value)) = trimmed.split_once('=') else {
                return Err(ConfigError::MalformedLine(trimmed.to_string()));
            };

            let key = key.trim();
            let value = raw_value.trim().trim_matches('"');

            if key == "profile" {
                profile = Some(Profile::parse(value)?);
            }
        }

        let profile = profile.ok_or(ConfigError::MissingProfile)?;
        Ok(Self { profile })
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: std::path::PathBuf,
        error: std::io::Error,
    },
    MalformedLine(String),
    MissingProfile,
    UnknownProfile(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(f, "failed to read {}: {error}", path.display())
            }
            Self::MalformedLine(line) => write!(f, "malformed config line: {line}"),
            Self::MissingProfile => write!(f, "missing `profile` setting"),
            Self::UnknownProfile(profile) => write!(f, "unknown profile `{profile}`"),
        }
    }
}

impl Error for ConfigError {}
