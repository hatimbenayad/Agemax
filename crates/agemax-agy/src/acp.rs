//! Live client and event parser for the Antigravity Agent Client Protocol (`agy acp`).
//! Strictly communicates over stdio JSON-RPC with the user's external `agy` binary —
//! zero embedded LLMs, prompts, or credential reads (`AGENTS.md` §2.3 & §2.4).

use crate::adapter::AntigravityAdapter;
use crate::profile::ProfileConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpToolCall {
    pub id: String,
    pub tool: String,
    pub ok: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpArtifact {
    pub title: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpEvent {
    Initialized {
        server_name: String,
        server_version: String,
    },
    SessionReady {
        session_id: String,
    },
    ToolCallStarted {
        id: String,
        tool: String,
    },
    ToolCallFinished {
        id: String,
        ok: bool,
    },
    ArtifactCreated {
        title: String,
        path: String,
    },
    MessageDelta {
        text: String,
    },
    StatusChanged {
        status: String,
    },
    PromptCompleted,
    ProcessExited {
        message: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct AcpCardState {
    pub server_banner: Option<String>,
    pub session_id: Option<String>,
    pub active_tools: Vec<AcpToolCall>,
    pub recent_artifacts: Vec<AcpArtifact>,
    pub latest_message: String,
    pub latest_status: Option<String>,
    pub is_turn_active: bool,
    pub exited_message: Option<String>,
}

impl AcpCardState {
    pub fn apply(&mut self, event: &AcpEvent) {
        match event {
            AcpEvent::Initialized {
                server_name,
                server_version,
            } => {
                self.server_banner = Some(format!("{server_name} v{server_version}"));
                self.latest_status = Some("Initialized".to_string());
            }
            AcpEvent::SessionReady { session_id } => {
                self.session_id = Some(session_id.clone());
                self.latest_status = Some("Ready".to_string());
            }
            AcpEvent::ToolCallStarted { id, tool } => {
                if !self.active_tools.iter().any(|t| t.id == *id) {
                    self.active_tools.push(AcpToolCall {
                        id: id.clone(),
                        tool: tool.clone(),
                        ok: None,
                    });
                }
            }
            AcpEvent::ToolCallFinished { id, ok } => {
                let _ = ok;
                self.active_tools.retain(|t| t.id != *id);
            }
            AcpEvent::ArtifactCreated { title, path } => {
                self.recent_artifacts.push(AcpArtifact {
                    title: title.clone(),
                    path: path.clone(),
                });
                if self.recent_artifacts.len() > 32 {
                    let drop_count = self.recent_artifacts.len() - 32;
                    self.recent_artifacts.drain(0..drop_count);
                }
            }
            AcpEvent::MessageDelta { text } => {
                self.latest_message.push_str(text);
                if self.latest_message.len() > 8192 {
                    let excess = self.latest_message.len() - 8192;
                    self.latest_message.drain(0..excess);
                }
            }
            AcpEvent::StatusChanged { status } => {
                self.latest_status = Some(status.clone());
            }
            AcpEvent::PromptCompleted => {
                self.is_turn_active = false;
                self.latest_status = Some("Ready".to_string());
            }
            AcpEvent::ProcessExited { message } => {
                self.is_turn_active = false;
                self.exited_message = Some(message.clone());
                self.latest_status = Some("Exited".to_string());
            }
        }
    }

    /// Derive structured agent state from current ACP turn & active tool calls (`REQ-087`).
    pub fn agent_status(&self) -> agemax_core::status::AgentStatus {
        if self.exited_message.is_some() {
            return agemax_core::status::AgentStatus::Idle;
        }
        if !self.active_tools.is_empty() {
            agemax_core::status::AgentStatus::RunningTool
        } else if self.is_turn_active {
            agemax_core::status::AgentStatus::Thinking
        } else if !self.latest_message.is_empty() {
            agemax_core::status::AgentStatus::Done
        } else {
            agemax_core::status::AgentStatus::Idle
        }
    }
}

pub fn build_acp_initialize_request(id: u64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": {
                "name": "agemax",
                "version": "0.1.0"
            }
        }
    })
    .to_string()
}

pub fn build_acp_session_new_request(id: u64, cwd: &Path) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "session/new",
        "params": {
            "cwd": cwd.to_string_lossy(),
            "mcpServers": []
        }
    })
    .to_string()
}

pub fn build_acp_prompt_request(id: u64, session_id: &str, text: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "session/prompt",
        "params": {
            "sessionId": session_id,
            "prompt": [{
                "type": "text",
                "text": text
            }]
        }
    })
    .to_string()
}

/// Parse one NDJSON event or JSON-RPC 2.0 line emitted by `agy acp`.
pub fn parse_acp_json_line(line: &str) -> Option<AcpEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let val: Value = serde_json::from_str(trimmed).ok()?;

    // 1. JSON-RPC 2.0 Response messages (initialize, session/new, session/prompt)
    if let Some(result) = val.get("result") {
        if let Some(info) = result.get("agentInfo") {
            let name = info
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Antigravity ACP")
                .to_string();
            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            return Some(AcpEvent::Initialized {
                server_name: name,
                server_version: version,
            });
        }
        if let Some(sid) = result.get("sessionId").and_then(|v| v.as_str()) {
            return Some(AcpEvent::SessionReady {
                session_id: sid.to_string(),
            });
        }
        if result.get("stopReason").is_some() {
            return Some(AcpEvent::PromptCompleted);
        }
    }

    if let Some(err) = val.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("ACP error")
            .to_string();
        return Some(AcpEvent::StatusChanged {
            status: format!("Error: {msg}"),
        });
    }

    // 2. JSON-RPC 2.0 Notifications (`session/update`)
    if val.get("method").and_then(|m| m.as_str()) == Some("session/update") {
        let update = val.get("params").and_then(|p| p.get("update"))?;
        let kind = update.get("sessionUpdate").and_then(|s| s.as_str())?;
        return match kind {
            "agent_message_chunk" => {
                let text = update
                    .get("content")
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())?
                    .to_string();
                Some(AcpEvent::MessageDelta { text })
            }
            "tool_call" => Some(AcpEvent::ToolCallStarted {
                id: update.get("toolCallId")?.as_str()?.to_string(),
                tool: update
                    .get("title")
                    .or_else(|| update.get("kind"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("tool")
                    .to_string(),
            }),
            "tool_call_update" => {
                let status = update.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if matches!(status, "completed" | "failed" | "cancelled") {
                    Some(AcpEvent::ToolCallFinished {
                        id: update.get("toolCallId")?.as_str()?.to_string(),
                        ok: status == "completed",
                    })
                } else {
                    Some(AcpEvent::StatusChanged {
                        status: format!("Tool: {status}"),
                    })
                }
            }
            "plan" => Some(AcpEvent::StatusChanged {
                status: "Updating plan".to_string(),
            }),
            _ => None,
        };
    }

    // 3. Flat NDJSON event schema (unit tests & custom hooks)
    let ev_type = val
        .get("type")
        .or_else(|| val.get("event"))
        .and_then(|v| v.as_str())?;

    match ev_type {
        "tool_call_started" => Some(AcpEvent::ToolCallStarted {
            id: val.get("id")?.as_str()?.to_string(),
            tool: val
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string(),
        }),
        "tool_call_finished" => Some(AcpEvent::ToolCallFinished {
            id: val.get("id")?.as_str()?.to_string(),
            ok: val.get("ok").and_then(|v| v.as_bool()).unwrap_or(true),
        }),
        "artifact_created" => Some(AcpEvent::ArtifactCreated {
            title: val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Artifact")
                .to_string(),
            path: val.get("path")?.as_str()?.to_string(),
        }),
        "message_delta" => Some(AcpEvent::MessageDelta {
            text: val.get("text")?.as_str()?.to_string(),
        }),
        "status_changed" => Some(AcpEvent::StatusChanged {
            status: val.get("status")?.as_str()?.to_string(),
        }),
        _ => None,
    }
}

/// Live stdio connection to an external `agy acp` child process bound to a Job Object on Windows.
pub struct LiveAcpSession {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<AcpEvent>,
    next_id: u64,
    cwd: std::path::PathBuf,
    session_requested: bool,
    #[cfg(windows)]
    _job_handle: Option<WindowsAcpJob>,
    pub state: AcpCardState,
    pub prompt_draft: String,
}

#[cfg(windows)]
struct WindowsAcpJob(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for WindowsAcpJob {}

#[cfg(windows)]
impl Drop for WindowsAcpJob {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn contain_pid_in_job(pid: u32) -> Option<WindowsAcpJob> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    unsafe {
        let job = CreateJobObjectW(None, None).ok()?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .is_ok();
        if !ok {
            let _ = CloseHandle(job);
            return None;
        }
        if let Ok(proc_h) = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid) {
            let _ = AssignProcessToJobObject(job, proc_h);
            let _ = CloseHandle(proc_h);
        }
        Some(WindowsAcpJob(job))
    }
}

impl LiveAcpSession {
    pub fn spawn(profile: &ProfileConfig, cwd: &Path) -> Result<Self, String> {
        let launch = AntigravityAdapter::build_acp_command(profile, cwd);
        let mut cmd = Command::new(&launch.program);
        cmd.args(&launch.args)
            .current_dir(&launch.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        for (k, v) in &launch.env {
            cmd.env(k, v);
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start {}: {e}", launch.program.display()))?;

        #[cfg(windows)]
        let job_handle = contain_pid_in_job(child.id());

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Missing stdin pipe for agy acp".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Missing stdout pipe for agy acp".to_string())?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line_res in reader.lines() {
                match line_res {
                    Ok(line) => {
                        if let Some(ev) = parse_acp_json_line(&line) {
                            if tx.send(ev).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AcpEvent::ProcessExited {
                            message: format!("ACP stream closed: {e}"),
                        });
                        break;
                    }
                }
            }
            let _ = tx.send(AcpEvent::ProcessExited {
                message: "ACP server exited".to_string(),
            });
        });

        let init_line = format!("{}\n", build_acp_initialize_request(1));
        stdin
            .write_all(init_line.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|e| format!("Failed to write ACP initialize request: {e}"))?;

        Ok(Self {
            child,
            stdin,
            rx,
            next_id: 2,
            cwd: cwd.to_path_buf(),
            session_requested: false,
            #[cfg(windows)]
            _job_handle: job_handle,
            state: AcpCardState::default(),
            prompt_draft: String::new(),
        })
    }

    pub fn poll(&mut self) -> bool {
        !self.poll_events().is_empty()
    }

    pub fn poll_events(&mut self) -> Vec<AcpEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            let just_initialized = matches!(ev, AcpEvent::Initialized { .. });
            self.state.apply(&ev);
            events.push(ev);

            if just_initialized && !self.session_requested {
                self.session_requested = true;
                let id = self.next_id;
                self.next_id += 1;
                let line = format!("{}\n", build_acp_session_new_request(id, &self.cwd));
                let _ = self.stdin.write_all(line.as_bytes());
                let _ = self.stdin.flush();
            }
        }
        events
    }

    pub fn send_prompt(&mut self, text: &str) -> Result<(), String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let sid = self
            .state
            .session_id
            .clone()
            .ok_or_else(|| "ACP session is still initializing...".to_string())?;
        let id = self.next_id;
        self.next_id += 1;
        let line = format!("{}\n", build_acp_prompt_request(id, &sid, trimmed));
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("Failed sending ACP prompt: {e}"))?;
        if !self.state.latest_message.is_empty() {
            self.state.latest_message.push_str("\n\n");
        }
        self.state
            .latest_message
            .push_str(&format!("You: {trimmed}\nAgent: "));
        self.state.is_turn_active = true;
        self.state.latest_status = Some("Thinking…".to_string());
        Ok(())
    }
}

impl Drop for LiveAcpSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
