mod db;
mod mcp;
mod notes;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp::serve().await
}
