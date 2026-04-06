use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    strz::main().await
}
