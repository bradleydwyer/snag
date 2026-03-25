use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct StreamInfo {
    pub stream_url: String,
    pub duration_secs: f64,
}

pub fn resolve_stream(url: &str) -> Result<StreamInfo> {
    let output = Command::new("yt-dlp")
        .args(["-g", "-f", "bestvideo[height<=720]/bestvideo/best", url])
        .output()
        .context("failed to run yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp failed: {stderr}");
    }

    let stream_url = String::from_utf8(output.stdout)
        .context("invalid utf-8 from yt-dlp")?
        .lines()
        .next()
        .context("yt-dlp returned no URL")?
        .to_string();

    let output = Command::new("yt-dlp")
        .args(["--print", "duration", url])
        .output()
        .context("failed to get duration")?;

    let duration_str = String::from_utf8(output.stdout)
        .context("invalid utf-8")?
        .trim()
        .to_string();

    let duration_secs: f64 = duration_str
        .parse()
        .unwrap_or_else(|_| parse_duration_hms(&duration_str).unwrap_or(60.0));

    Ok(StreamInfo {
        stream_url,
        duration_secs,
    })
}

fn parse_duration_hms(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].parse().ok()?;
            Some(m * 60.0 + s)
        }
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        _ => None,
    }
}

pub fn download_video(url: &str, output_dir: &Path) -> Result<PathBuf> {
    let output_path = output_dir.join("video.mp4");

    eprintln!("Downloading video...");

    let status = Command::new("yt-dlp")
        .args([
            "-f",
            "best[height<=720]/best",
            "--merge-output-format",
            "mp4",
            "-o",
            output_path.to_str().unwrap(),
            url,
        ])
        .status()
        .context("failed to run yt-dlp")?;

    if !status.success() {
        bail!("yt-dlp download failed");
    }

    Ok(output_path)
}
