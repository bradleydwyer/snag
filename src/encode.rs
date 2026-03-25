use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn encode_gif(
    stream_url: &str,
    start_secs: f64,
    end_secs: f64,
    width: u32,
    fps: u32,
    output_path: &str,
) -> Result<()> {
    let start = format!("{start_secs:.2}");
    let duration = format!("{:.2}", end_secs - start_secs);
    let scale = format!("fps={fps},scale={width}:-1:flags=lanczos");

    eprintln!(
        "Encoding GIF: {start}s to {:.2}s ({duration}s) at {fps}fps, {width}px wide...",
        end_secs
    );

    let tmpdir = tempfile::tempdir().context("failed to create temp dir")?;
    let pattern = tmpdir.path().join("frame_%05d.png");

    // Extract frames to temp dir
    let ffmpeg_status = Command::new("ffmpeg")
        .args([
            "-ss", &start,
            "-t", &duration,
            "-i", stream_url,
            "-vf", &scale,
            "-y",
            "-loglevel", "error",
            pattern.to_str().unwrap(),
        ])
        .status()
        .context("failed to run ffmpeg")?;

    if !ffmpeg_status.success() {
        bail!("ffmpeg frame extraction failed");
    }

    // Collect frame paths
    let mut frames: Vec<_> = std::fs::read_dir(tmpdir.path())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "png"))
        .collect();
    frames.sort();

    if frames.is_empty() {
        bail!("no frames extracted for GIF");
    }

    eprintln!("Encoding {} frames with gifski...", frames.len());

    let fps_str = fps.to_string();
    let width_str = width.to_string();

    let frame_args: Vec<&str> = frames.iter().filter_map(|p| p.to_str()).collect();

    let mut cmd = Command::new("gifski");
    cmd.args(["--fps", &fps_str, "-W", &width_str, "-o", output_path]);
    cmd.args(&frame_args);

    let gifski_status = cmd.status().context("failed to run gifski")?;

    if !gifski_status.success() {
        bail!("gifski encoding failed");
    }

    eprintln!("GIF saved to {output_path}");
    Ok(())
}
