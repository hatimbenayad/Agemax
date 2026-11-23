use agemax_core::status::AgentStatus;

/// Events parsed from ANSI / OSC terminal escape streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentOscEvent {
    /// Agent status transition signaled via OSC 7501.
    StatusChange {
        status: AgentStatus,
        message: Option<String>,
    },
    /// General terminal notification (OSC 9 / OSC 777 / OSC 99).
    Notification { title: String, message: String },
    /// Terminal audible / visual bell (`\x07` outside an escape sequence).
    Bell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Ground,
    Escape,
    Osc,
    OscEscape,
}

/// Streaming parser for ANSI OSC escape sequences and terminal bell signals.
#[derive(Debug, Clone)]
pub struct OscParser {
    state: ParserState,
    osc_buf: Vec<u8>,
}

impl Default for OscParser {
    fn default() -> Self {
        Self::new()
    }
}

impl OscParser {
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            osc_buf: Vec::with_capacity(256),
        }
    }

    /// Process a stream of bytes and return any extracted agent events.
    pub fn parse(&mut self, bytes: &[u8]) -> Vec<AgentOscEvent> {
        let mut events = Vec::new();

        for &b in bytes {
            match self.state {
                ParserState::Ground => match b {
                    0x07 => {
                        events.push(AgentOscEvent::Bell);
                    }
                    0x1B => {
                        self.state = ParserState::Escape;
                    }
                    _ => {}
                },
                ParserState::Escape => match b {
                    b']' => {
                        self.osc_buf.clear();
                        self.state = ParserState::Osc;
                    }
                    0x1B => {
                        // Successive escape byte
                        self.state = ParserState::Escape;
                    }
                    _ => {
                        self.state = ParserState::Ground;
                    }
                },
                ParserState::Osc => match b {
                    0x07 => {
                        // BEL terminator
                        if let Some(event) = self.dispatch_osc() {
                            events.push(event);
                        }
                        self.osc_buf.clear();
                        self.state = ParserState::Ground;
                    }
                    0x1B => {
                        self.state = ParserState::OscEscape;
                    }
                    _ => {
                        if self.osc_buf.len() < 4096 {
                            self.osc_buf.push(b);
                        } else {
                            // Buffer overflow guard: reset to ground
                            self.osc_buf.clear();
                            self.state = ParserState::Ground;
                        }
                    }
                },
                ParserState::OscEscape => match b {
                    b'\\' => {
                        // String Terminator (ST = ESC \)
                        if let Some(event) = self.dispatch_osc() {
                            events.push(event);
                        }
                        self.osc_buf.clear();
                        self.state = ParserState::Ground;
                    }
                    0x07 => {
                        if let Some(event) = self.dispatch_osc() {
                            events.push(event);
                        }
                        self.osc_buf.clear();
                        self.state = ParserState::Ground;
                    }
                    b']' => {
                        // Aborted previous OSC and started new one
                        self.osc_buf.clear();
                        self.state = ParserState::Osc;
                    }
                    _ => {
                        // ESC was literal or followed by something else inside OSC
                        if self.osc_buf.len() + 2 <= 4096 {
                            self.osc_buf.push(0x1B);
                            self.osc_buf.push(b);
                        }
                        self.state = ParserState::Osc;
                    }
                },
            }
        }

        events
    }

    fn dispatch_osc(&self) -> Option<AgentOscEvent> {
        let text = String::from_utf8_lossy(&self.osc_buf);

        if let Some(payload) = text.strip_prefix("7501;") {
            return parse_osc_7501(payload);
        }

        if let Some(payload) = text.strip_prefix("9;") {
            let message = payload.trim().to_string();
            return Some(AgentOscEvent::Notification {
                title: "Terminal Notification".to_string(),
                message,
            });
        }

        if let Some(payload) = text.strip_prefix("777;") {
            return parse_osc_777(payload);
        }

        None
    }
}

fn parse_osc_7501(payload: &str) -> Option<AgentOscEvent> {
    let mut status = None;
    let mut message = None;

    for part in payload.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            let key = k.trim().to_lowercase();
            let val = v.trim();

            match key.as_str() {
                "status" | "state" => {
                    status = match val.to_lowercase().as_str() {
                        "waiting" | "waiting_for_input" | "waiting-for-input" | "prompt" => {
                            Some(AgentStatus::WaitingForInput)
                        }
                        "working" | "busy" | "running" => Some(AgentStatus::Working),
                        "done" | "completed" | "success" => Some(AgentStatus::Done),
                        "failed" | "error" => Some(AgentStatus::Failed),
                        "idle" => Some(AgentStatus::Idle),
                        _ => None,
                    };
                }
                "prompt" | "summary" | "message" | "error" | "title" if !val.is_empty() => {
                    message = Some(val.to_string());
                }
                _ => {}
            }
        }
    }

    status.map(|s| AgentOscEvent::StatusChange { status: s, message })
}

fn parse_osc_777(payload: &str) -> Option<AgentOscEvent> {
    // Format: "notify;Title;Message" or "notify;Message"
    let parts: Vec<&str> = payload.split(';').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    if parts[0].eq_ignore_ascii_case("notify") {
        let (title, message) = match parts.len() {
            1 => ("Notification".to_string(), String::new()),
            2 => ("Notification".to_string(), parts[1].to_string()),
            _ => (parts[1].to_string(), parts[2..].join("; ")),
        };
        Some(AgentOscEvent::Notification { title, message })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_7501_bel_terminated() {
        let mut parser = OscParser::new();
        let input = b"Hello \x1b]7501;status=waiting;prompt=Approve write\x07 World";
        let events = parser.parse(input);

        assert_eq!(
            events,
            vec![AgentOscEvent::StatusChange {
                status: AgentStatus::WaitingForInput,
                message: Some("Approve write".to_string()),
            }]
        );
    }

    #[test]
    fn test_osc_7501_st_terminated() {
        let mut parser = OscParser::new();
        let input = b"\x1b]7501;status=done;summary=Build complete\x1b\\";
        let events = parser.parse(input);

        assert_eq!(
            events,
            vec![AgentOscEvent::StatusChange {
                status: AgentStatus::Done,
                message: Some("Build complete".to_string()),
            }]
        );
    }

    #[test]
    fn test_osc_7501_working_no_message() {
        let mut parser = OscParser::new();
        let input = b"\x1b]7501;status=working\x07";
        let events = parser.parse(input);

        assert_eq!(
            events,
            vec![AgentOscEvent::StatusChange {
                status: AgentStatus::Working,
                message: None,
            }]
        );
    }

    #[test]
    fn test_osc_7501_chunked_streaming() {
        let mut parser = OscParser::new();
        let chunk1 = b"abc \x1b]7501;status=";
        let chunk2 = b"failed;error=Compile failed\x07 xyz";

        let events1 = parser.parse(chunk1);
        assert!(events1.is_empty());

        let events2 = parser.parse(chunk2);
        assert_eq!(
            events2,
            vec![AgentOscEvent::StatusChange {
                status: AgentStatus::Failed,
                message: Some("Compile failed".to_string()),
            }]
        );
    }

    #[test]
    fn test_bell_detection() {
        let mut parser = OscParser::new();
        let input = b"Output line 1\x07Output line 2";
        let events = parser.parse(input);

        assert_eq!(events, vec![AgentOscEvent::Bell]);
    }

    #[test]
    fn test_osc_9_notification() {
        let mut parser = OscParser::new();
        let input = b"\x1b]9;Agent finished task\x07";
        let events = parser.parse(input);

        assert_eq!(
            events,
            vec![AgentOscEvent::Notification {
                title: "Terminal Notification".to_string(),
                message: "Agent finished task".to_string(),
            }]
        );
    }

    #[test]
    fn test_osc_777_notification() {
        let mut parser = OscParser::new();
        let input = b"\x1b]777;notify;Antigravity;Tests passing\x07";
        let events = parser.parse(input);

        assert_eq!(
            events,
            vec![AgentOscEvent::Notification {
                title: "Antigravity".to_string(),
                message: "Tests passing".to_string(),
            }]
        );
    }
}
