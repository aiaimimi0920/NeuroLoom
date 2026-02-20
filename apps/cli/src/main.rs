//! NeuroLoom CLI - 命令行交互接口

use std::io::{self, BufRead, Write};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "neuroloom_cli=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("NeuroLoom CLI v0.1.0");
    println!("Type 'help' for available commands, 'quit' to exit.");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("nl> ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0];

        match command {
            "help" => {
                println!("Available commands:");
                println!("  help          - Show this help message");
                println!("  status        - Show system status");
                println!("  nodes         - List workspace nodes");
                println!("  actors        - List active actors");
                println!("  memory        - Show memory statistics");
                println!("  clear         - Clear the screen");
                println!("  quit / exit   - Exit the CLI");
            }
            "status" => {
                println!("System Status:");
                println!("  Daemon: Running");
                println!("  Actors: 0 active");
                println!("  Memory: 0 entries");
            }
            "nodes" => {
                println!("Workspace Nodes: (none)");
            }
            "actors" => {
                println!("Active Actors: (none)");
            }
            "memory" => {
                println!("Memory Statistics:");
                println!("  Total entries: 0");
                println!("  Cache size: 0 bytes");
            }
            "clear" => {
                print!("\x1B[2J\x1B[1;1H");
            }
            "quit" | "exit" => {
                println!("Goodbye!");
                break;
            }
            _ => {
                println!("Unknown command: {}", command);
                println!("Type 'help' for available commands.");
            }
        }
    }

    Ok(())
}
