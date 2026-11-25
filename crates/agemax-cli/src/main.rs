//! CLI client for controlling a running Age Max instance over local named pipe.

use agemax_ipc::client::IpcClient;
use agemax_ipc::security::get_default_pipe_name;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "agemax",
    about = "Age Max CLI - Scriptable controller for the Age Max agent multiplexer",
    version = "0.1.0"
)]
struct Cli {
    #[arg(long, help = "Override named pipe endpoint")]
    pipe: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(subcommand, about = "Manage workspaces")]
    Workspace(WorkspaceCommands),

    #[command(about = "Split the active terminal pane")]
    Split {
        #[arg(short, long, default_value = "right")]
        direction: SplitDir,
    },

    #[command(about = "Focus a targeted pane")]
    Focus {
        #[arg(help = "Target pane UUID")]
        pane_id: String,
    },

    #[command(about = "Send keystrokes or text to a pane")]
    SendKeys {
        #[arg(help = "Keystrokes / text to dispatch")]
        keys: String,

        #[arg(short, long, help = "Target pane UUID (defaults to active)")]
        pane: Option<String>,
    },

    #[command(about = "Read the visible terminal screen text")]
    ReadScreen {
        #[arg(short, long, help = "Target pane UUID (defaults to active)")]
        pane: Option<String>,
    },

    #[command(about = "Emit a notification and visual attention cue")]
    Notify {
        #[arg(help = "Notification body text")]
        message: String,

        #[arg(short, long, default_value = "Agent Notification")]
        title: String,

        #[arg(short, long)]
        status: Option<String>,
    },

    #[command(about = "Set the agent status of a pane")]
    Status {
        #[arg(help = "New status: idle, working, waiting, done, failed")]
        status: String,

        #[arg(short, long)]
        message: Option<String>,

        #[arg(short, long)]
        pane: Option<String>,
    },

    #[command(about = "Resume an agent conversation")]
    Resume {
        #[arg(help = "Conversation ID to resume")]
        conversation_id: Option<String>,

        #[arg(short, long)]
        pane: Option<String>,
    },

    #[command(subcommand, about = "List or inspect profiles")]
    Profiles(ProfileCommands),

    #[command(about = "Focus the running Age Max window and optionally open a folder")]
    Activate {
        #[arg(short, long, help = "Directory to focus or open as a workspace")]
        cwd: Option<String>,
    },

    #[command(subcommand, about = "Control embedded WebView2 browser preview panes")]
    Browser(BrowserCommands),
}

#[derive(Subcommand, Debug)]
enum BrowserCommands {
    #[command(about = "Open a URL in a browser preview pane")]
    Open {
        #[arg(help = "URL to navigate to")]
        url: String,
        #[arg(short, long, help = "Optional pane UUID to reuse/target")]
        pane: Option<String>,
    },
    #[command(about = "Capture a PNG screenshot of a browser preview pane")]
    Screenshot {
        #[arg(short, long, help = "Target browser pane UUID (defaults to active)")]
        pane: Option<String>,
    },
    #[command(about = "Evaluate JavaScript inside a browser preview pane")]
    Eval {
        #[arg(help = "JavaScript expression or statements")]
        script: String,
        #[arg(short, long, help = "Target browser pane UUID (defaults to active)")]
        pane: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum WorkspaceCommands {
    #[command(about = "List all active workspaces")]
    List,
    #[command(about = "Create a new workspace")]
    Create {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        cwd: Option<String>,
    },
    #[command(about = "Close a workspace")]
    Close {
        #[arg(help = "Workspace UUID")]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ProfileCommands {
    #[command(about = "List configured profiles")]
    List,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum SplitDir {
    Right,
    Down,
    Horizontal,
    Vertical,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let pipe_name = cli.pipe.unwrap_or_else(get_default_pipe_name);
    let client = IpcClient::new(&pipe_name);

    match cli.command {
        Commands::Workspace(cmd) => match cmd {
            WorkspaceCommands::List => {
                let res = client
                    .call("workspace.list", serde_json::json!({}))
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
            WorkspaceCommands::Create { name, cwd } => {
                let res = client
                    .call(
                        "workspace.create",
                        serde_json::json!({ "name": name, "cwd": cwd }),
                    )
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
            WorkspaceCommands::Close { id } => {
                let res = client
                    .call("workspace.close", serde_json::json!({ "id": id }))
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        },
        Commands::Split { direction } => {
            let dir_str = match direction {
                SplitDir::Right | SplitDir::Horizontal => "right",
                SplitDir::Down | SplitDir::Vertical => "down",
            };
            let res = client
                .call("pane.split", serde_json::json!({ "direction": dir_str }))
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Focus { pane_id } => {
            let res = client
                .call("pane.focus", serde_json::json!({ "pane_id": pane_id }))
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::SendKeys { keys, pane } => {
            let res = client
                .call(
                    "pane.send_keys",
                    serde_json::json!({ "keys": keys, "pane_id": pane }),
                )
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::ReadScreen { pane } => {
            let res = client
                .call("pane.read_screen", serde_json::json!({ "pane_id": pane }))
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            if let Some(text) = res.as_str() {
                println!("{text}");
            } else {
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        }
        Commands::Notify {
            message,
            title,
            status,
        } => {
            let res = client
                .call(
                    "notify",
                    serde_json::json!({ "title": title, "message": message, "status": status }),
                )
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Status {
            status,
            message,
            pane,
        } => {
            let res = client
                .call(
                    "status.set",
                    serde_json::json!({ "status": status, "message": message, "pane_id": pane }),
                )
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Resume {
            conversation_id,
            pane,
        } => {
            let res = client
                .call(
                    "session.resume",
                    serde_json::json!({ "conversation_id": conversation_id, "pane_id": pane }),
                )
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Profiles(cmd) => match cmd {
            ProfileCommands::List => {
                let res = client
                    .call("profile.list", serde_json::json!({}))
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        },
        Commands::Activate { cwd } => {
            let res = client
                .call("app.activate", serde_json::json!({ "cwd": cwd }))
                .await
                .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
            println!("{}", serde_json::to_string_pretty(&res)?);
        }
        Commands::Browser(cmd) => match cmd {
            BrowserCommands::Open { url, pane } => {
                let res = client
                    .call(
                        "browser.open",
                        serde_json::json!({ "url": url, "pane_id": pane }),
                    )
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
            BrowserCommands::Screenshot { pane } => {
                let res = client
                    .call("browser.screenshot", serde_json::json!({ "pane_id": pane }))
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
            BrowserCommands::Eval { script, pane } => {
                let res = client
                    .call(
                        "browser.eval",
                        serde_json::json!({ "script": script, "pane_id": pane }),
                    )
                    .await
                    .with_context(|| format!("Failed to connect to Age Max on {pipe_name}"))?;
                println!("{}", serde_json::to_string_pretty(&res)?);
            }
        },
    }

    Ok(())
}
