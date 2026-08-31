//! Errors from reading and writing SOP sources.
//!
//! The dividing line between an error here and a
//! [`Diagnostic`](crate::Diagnostic) is whether a value could still be
//! produced. If reading yielded a SOP, anything noteworthy is a diagnostic and
//! the caller may ignore it. If reading could not yield a SOP, it is an error
//! and the caller cannot ignore it. Nothing lives in both places.

use std::path::PathBuf;

use thiserror::Error;

use crate::diagnostic::Location;

/// Reading a SOP from disk or from source text failed.
#[derive(Debug, Error)]
pub enum ReadError {
    /// The SOP directory, or a file inside it, could not be read.
    #[error("could not read {path}: {source}")]
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// A required file is missing from the SOP directory.
    ///
    /// A SOP is a directory containing both `SOP.toml` and `SOP.md`; neither is
    /// optional, and a directory with only one is more likely a mistake than an
    /// intentional partial definition.
    #[error("{path} is not a SOP directory: {missing} is missing")]
    NotASopDirectory {
        /// The directory inspected.
        path: PathBuf,
        /// The file that should have been there.
        missing: &'static str,
    },

    /// `SOP.toml` is not well-formed TOML.
    #[error("SOP.toml is not valid TOML: {source}")]
    ManifestSyntax {
        /// The underlying parse failure, which carries its own span.
        #[source]
        source: toml::de::Error,
    },

    /// `SOP.toml` parsed as TOML but does not describe a SOP manifest.
    #[error("SOP.toml is not a valid manifest at {location}: {reason}")]
    ManifestShape {
        /// Where the problem is.
        location: Location,
        /// What is wrong.
        reason: String,
    },

    /// `SOP.md` could not be parsed into steps.
    #[error("SOP.md could not be parsed at {location}: {reason}")]
    Steps {
        /// Where the problem is.
        location: Location,
        /// What is wrong.
        reason: String,
    },
}

impl ReadError {
    /// Convenience for the common `io::Error` wrap.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Convenience for a manifest shape failure.
    pub fn manifest_shape(location: Location, reason: impl Into<String>) -> Self {
        Self::ManifestShape {
            location,
            reason: reason.into(),
        }
    }

    /// Convenience for a step parse failure.
    pub fn steps(location: Location, reason: impl Into<String>) -> Self {
        Self::Steps {
            location,
            reason: reason.into(),
        }
    }
}

/// Writing a SOP to disk failed.
#[derive(Debug, Error)]
pub enum WriteError {
    /// A file or directory could not be written.
    #[error("could not write {path}: {source}")]
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// The manifest could not be serialized to TOML.
    ///
    /// Reaching this means a SOP was constructed that cannot be represented in
    /// the format, which is a bug in whatever built it rather than bad input.
    #[error("could not serialize SOP.toml: {source}")]
    ManifestSerialize {
        /// The underlying failure.
        #[source]
        source: toml::ser::Error,
    },
}

impl WriteError {
    /// Convenience for the common `io::Error` wrap.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_errors_name_the_path() {
        let e = ReadError::io(
            "/tmp/nope/SOP.toml",
            std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"),
        );
        let msg = e.to_string();
        assert!(msg.contains("/tmp/nope/SOP.toml"), "{msg}");
    }

    #[test]
    fn missing_file_error_names_the_file() {
        let e = ReadError::NotASopDirectory {
            path: "/tmp/thing".into(),
            missing: "SOP.md",
        };
        let msg = e.to_string();
        assert!(msg.contains("SOP.md"), "{msg}");
        assert!(msg.contains("/tmp/thing"), "{msg}");
    }

    #[test]
    fn step_errors_carry_a_location() {
        let e = ReadError::steps(
            Location::Step {
                number: 3,
                line: Some(12),
            },
            "expected a step title",
        );
        let msg = e.to_string();
        assert!(msg.contains("step 3"), "{msg}");
        assert!(msg.contains("expected a step title"), "{msg}");
    }

    #[test]
    fn source_chain_is_preserved() {
        use std::error::Error as _;
        let e = ReadError::io(
            "/x",
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        );
        assert!(e.source().is_some(), "the io::Error must remain reachable");
    }
}
