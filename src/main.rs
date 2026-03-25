mod encode;
mod pipeline;
mod server;

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::process::Command;

#[derive(Parser)]
#[command(name = "snag", about = "Snag a GIF from a YouTube video")]
struct Cli {
    /// YouTube URL (required for direct encode mode)
    url: Option<String>,

    /// Output GIF path
    #[arg(short, long, default_value = "output.gif")]
    output: String,

    /// GIF width in pixels
    #[arg(short = 'W', long, default_value_t = 480)]
    width: u32,

    /// GIF frames per second
    #[arg(long, default_value_t = 15)]
    fps: u32,

    /// Start time in seconds (skip web UI, encode directly)
    #[arg(short, long)]
    start: Option<f64>,

    /// End time in seconds (skip web UI, encode directly)
    #[arg(short, long)]
    end: Option<f64>,

    /// Port for the web UI
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

fn check_dep(name: &str, install_hint: &str) -> Result<()> {
    let status = Command::new("which")
        .arg(name)
        .output()
        .context(format!("failed to check for {name}"))?;

    if !status.status.success() {
        bail!("{name} not found in PATH. Install with: {install_hint}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    check_dep("yt-dlp", "brew install yt-dlp")?;
    check_dep("ffmpeg", "brew install ffmpeg")?;
    // gifski is checked lazily at encode time since it's only needed then

    // Direct encode mode: skip web UI when URL + start + end are given
    if let (Some(url), Some(start), Some(end)) = (&cli.url, cli.start, cli.end) {
        if start >= end {
            bail!("--start must be less than --end");
        }
        eprintln!("Resolving stream URL...");
        let info = pipeline::resolve_stream(url)?;
        eprintln!("Duration: {:.0}s", info.duration_secs);
        encode::encode_gif(
            &info.stream_url,
            start,
            end,
            cli.width,
            cli.fps,
            &cli.output,
        )?;
        return Ok(());
    }

    // Web UI mode
    let port = cli.port;
    eprintln!("Starting snag web UI...");
    let _ = open::that(format!("http://localhost:{port}"));
    server::run(port, cli.width, cli.fps).await?;

    Ok(())
}
