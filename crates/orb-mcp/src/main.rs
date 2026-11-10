//! The `orb-mcp` binary: the OpenRB-338 tools over stdio, for an MCP client to start.
//!
//! Nothing goes to stdout but the protocol: the diagnostics of a render (the warnings `orb`
//! prints) are stderr's, as they are for `orb` itself.

use orb_mcp::Orb;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Orb::new().serve(stdio()).await?;
    // Runs until the client closes stdin, then shuts the server down.
    service.waiting().await?;
    Ok(())
}
