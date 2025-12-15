mod executor;
mod request;
mod security;
mod server;
mod tools;

use rmcp::{transport::stdio, ServiceExt};
use server::CommandRunnerServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    CommandRunnerServer::new().serve(stdio()).await?.waiting().await?;
    Ok(())
}
