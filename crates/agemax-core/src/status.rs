use serde::{Deserialize, Serialize};

/// High-level operational status of an agent session or surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AgentStatus {
    #[default]
    Idle,
    Working,
    Thinking,
    RunningTool,
    WaitingForInput,
    Done,
    Failed,
}

impl AgentStatus {
    pub fn is_attention_needed(&self) -> bool {
        matches!(self, Self::WaitingForInput | Self::Done | Self::Failed)
    }

    pub fn is_working(&self) -> bool {
        matches!(self, Self::Working | Self::Thinking | Self::RunningTool)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Working => "Working",
            Self::Thinking => "Thinking",
            Self::RunningTool => "Running tool",
            Self::WaitingForInput => "Waiting for input",
            Self::Done => "Done",
            Self::Failed => "Failed",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Idle => "o",
            Self::Working => "*",
            Self::Thinking => "~",
            Self::RunningTool => "#",
            Self::WaitingForInput => "!",
            Self::Done => "+",
            Self::Failed => "x",
        }
    }
}
