use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    structurizr_cli::main().await
}
