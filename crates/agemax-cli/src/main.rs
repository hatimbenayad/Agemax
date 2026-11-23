//! CLI client for controlling a running Age Max instance over local named pipe.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "agemax",
    about = "Age Max CLI - Control the Age Max multiplexer"
)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let _cli = Cli::parse();
    println!("Age Max CLI");
}
