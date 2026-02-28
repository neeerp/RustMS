use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("missing harness config file at `{path}`")]
    MissingConfigFile { path: String },

    #[error("failed to read harness config file `{path}`: {source}")]
    ConfigRead {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse harness config file `{path}`: {message}")]
    ConfigParse { path: String, message: String },

    #[error("invalid socket address for {name} `{value}`: {source}")]
    InvalidAddress {
        name: &'static str,
        value: String,
        #[source]
        source: std::net::AddrParseError,
    },

    #[error("io error during {phase} against {endpoint}: {source}")]
    Io {
        phase: &'static str,
        endpoint: String,
        #[source]
        source: io::Error,
    },

    #[error("protocol error during {phase} against {endpoint}: {message}")]
    Protocol {
        phase: &'static str,
        endpoint: String,
        message: String,
    },

    #[error("fixture error during {phase}: {message}")]
    Fixture {
        phase: &'static str,
        message: String,
    },
}

impl HarnessError {
    pub fn io(phase: &'static str, endpoint: impl ToString, source: io::Error) -> Self {
        Self::Io {
            phase,
            endpoint: endpoint.to_string(),
            source,
        }
    }

    pub fn protocol(
        phase: &'static str,
        endpoint: impl ToString,
        message: impl Into<String>,
    ) -> Self {
        Self::Protocol {
            phase,
            endpoint: endpoint.to_string(),
            message: message.into(),
        }
    }

    pub fn fixture(phase: &'static str, message: impl Into<String>) -> Self {
        Self::Fixture {
            phase,
            message: message.into(),
        }
    }
}
