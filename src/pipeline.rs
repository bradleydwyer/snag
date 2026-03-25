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

pub fn preview_fps(duration_secs: f64) -> f64 {
    if duration_secs > 300.0 {
        2.0
    } else if duration_secs > 120.0 {
        5.0
    } else {
        10.0
    }
}

pub fn extract_preview_frames(
    stream_url: &str,
    tmpdir: &Path,
    duration_secs: f64,
    width: u32,
) -> Result<Vec<PathBuf>> {
    let fps = preview_fps(duration_secs);
    let fps = format!("{fps}");
    let scale = format!("fps={fps},scale={width}:-1");
    let pattern = tmpdir.join("frame_%05d.png");

    eprintln!("Extracting preview frames...");

    let status = Command::new("ffmpeg")
        .args([
            "-i",
            stream_url,
            "-vf",
            &scale,
            "-q:v",
            "2",
            pattern.to_str().unwrap(),
            "-y",
            "-loglevel",
            "error",
        ])
        .status()
        .context("failed to run ffmpeg")?;

    if !status.success() {
        bail!("ffmpeg frame extraction failed");
    }

    let mut frames: Vec<PathBuf> = std::fs::read_dir(tmpdir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "png"))
        .collect();

    frames.sort();

    if frames.is_empty() {
        bail!("no frames extracted");
    }

    eprintln!("Extracted {} preview frames", frames.len());
    Ok(frames)
}
