mod encode;
mod pipeline;
mod player;
mod tui;

use anyhow::{Context, Result, bail};
use clap::Parser;
use image::ImageReader;
use std::process::Command;

#[derive(Parser)]
#[command(name = "snag", about = "Snag a GIF from a YouTube video")]
struct Cli {
    /// YouTube URL
    url: String,

    /// Output GIF path
    #[arg(short, long, default_value = "output.gif")]
    output: String,

    /// GIF width in pixels
    #[arg(short = 'W', long, default_value_t = 480)]
    width: u32,

    /// GIF frames per second
    #[arg(long, default_value_t = 15)]
    fps: u32,

    /// Start time in seconds (skip scrubber)
    #[arg(short, long)]
    start: Option<f64>,

    /// End time in seconds (skip scrubber)
    #[arg(short, long)]
    end: Option<f64>,
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    check_dep("yt-dlp", "brew install yt-dlp")?;
    check_dep("ffmpeg", "brew install ffmpeg")?;
    check_dep("gifski", "brew install gifski")?;

    eprintln!("Resolving stream URL...");
    let info = pipeline::resolve_stream(&cli.url)?;
    eprintln!("Duration: {:.0}s", info.duration_secs);

    // Direct encode mode: skip scrubber when both start and end are given
    if let (Some(start), Some(end)) = (cli.start, cli.end) {
        if start >= end {
            bail!("--start must be less than --end");
        }
        encode::encode_gif(
            &info.stream_url,
            start,
            end,
            cli.width,
            cli.fps,
            &cli.output,
        )?;
        show_gif(&cli.output);
        return Ok(());
    }

    let tmpdir = tempfile::tempdir().context("failed to create temp dir")?;
    let frame_paths = pipeline::extract_preview_frames(
        &info.stream_url,
        tmpdir.path(),
        info.duration_secs,
        cli.width,
    )?;

    let preview_fps = pipeline::preview_fps(info.duration_secs);

    let frames: Vec<image::DynamicImage> = frame_paths
        .iter()
        .map(|p| {
            let reader = ImageReader::open(p)
                .with_context(|| format!("failed to open frame: {}", p.display()))?;
            reader
                .decode()
                .with_context(|| format!("failed to decode frame: {}", p.display()))
        })
        .collect::<Result<Vec<_>>>()?;

    eprintln!("Loaded {} frames. Launching scrubber...", frames.len());

    let mut player = player::Player::new(frames, info.duration_secs, preview_fps);

    match tui::run(&mut player)? {
        tui::TuiResult::Encode => {
            let start = player.start_time().unwrap();
            let end = player.end_time().unwrap();
            encode::encode_gif(
                &info.stream_url,
                start,
                end,
                cli.width,
                cli.fps,
                &cli.output,
            )?;
            show_gif(&cli.output);
        }
        tui::TuiResult::Quit => {
            eprintln!("Cancelled.");
        }
    }

    Ok(())
}

fn show_gif(path: &str) {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
    let url = format!("file://{}", abs.display());
    let _ = Command::new("open").args(["-a", "Safari", &url]).status();
}
