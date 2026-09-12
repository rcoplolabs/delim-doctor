mod scan;
mod server;
mod writer;

use std::path::PathBuf;

use rmcp::ServiceExt;
use rmcp::transport::stdio;
use server::DelimDoctorServer;

fn parse_workspace_root() -> anyhow::Result<PathBuf> {
    let args: Vec<String> = std::env::args().collect();

    // --version takes priority over all other arguments.
    if args.iter().any(|a| a == "--version") {
        println!("delim-doctor {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    let mut workspace_root: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--workspace-root" => {
                i += 1;
                if i >= args.len() {
                    anyhow::bail!("--workspace-root requires a value");
                }
                workspace_root = Some(PathBuf::from(&args[i]));
            }
            other => {
                anyhow::bail!("unknown argument: {}", other);
            }
        }
        i += 1;
    }

    let root = match workspace_root {
        Some(r) => r,
        None => std::env::current_dir().map_err(|e| {
            anyhow::anyhow!("no --workspace-root given and cannot determine cwd: {e}")
        })?,
    };

    if !root.exists() {
        anyhow::bail!("workspace root does not exist: {}", root.display());
    }

    let canonical = std::fs::canonicalize(&root)
        .map_err(|e| anyhow::anyhow!("failed to canonicalize workspace root: {}", e))?;

    Ok(canonical)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let workspace_root = parse_workspace_root()?;

    let service = DelimDoctorServer { workspace_root }
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("failed to start MCP server: {}", e))?;

    service.waiting().await?;

    Ok(())
}
