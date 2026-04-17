use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartAgentRunError {
    ReserveRun(String),
    AttachRunHandle(String),
}

impl fmt::Display for StartAgentRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReserveRun(message) | Self::AttachRunHandle(message) => f.write_str(message),
        }
    }
}

impl From<StartAgentRunError> for String {
    fn from(error: StartAgentRunError) -> Self {
        error.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendPromptError {
    EmptyPrompt,
    RunNotActive,
}

impl fmt::Display for SendPromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPrompt => f.write_str("prompt is empty"),
            Self::RunNotActive => f.write_str("agent run is not active"),
        }
    }
}

impl From<SendPromptError> for String {
    fn from(error: SendPromptError) -> Self {
        error.to_string()
    }
}
