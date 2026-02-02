use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
  mises_unix_server::cli::run::run().await
}
