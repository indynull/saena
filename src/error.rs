//! Failures a sitting verb can return.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotFound,
    NotReady,
    NotWork,
    NotClaimed,
    NotTerminal,
    Terminal,
    Conflict,
    ToolDump,
    Deleted,
    MissingBytes,
    Digest,
    Usage,
    Method,
    NoBinary,
    Host,
    Timeout,
}

impl Error {
    pub fn token(self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::NotReady => "not_ready",
            Self::NotWork => "not_work",
            Self::NotClaimed => "not_claimed",
            Self::NotTerminal => "not_terminal",
            Self::Terminal => "terminal",
            Self::Conflict => "conflict",
            Self::ToolDump => "tool_dump",
            Self::Deleted => "deleted",
            Self::MissingBytes => "missing_bytes",
            Self::Digest => "digest",
            Self::Usage => "usage",
            Self::Method => "method",
            Self::NoBinary => "no_binary",
            Self::Host => "host",
            Self::Timeout => "timeout",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
