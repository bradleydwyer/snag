# snag

Snag a GIF from a YouTube video. Pick a clip in your browser, or encode it directly from the command line.

snag uses yt-dlp to fetch videos, FFmpeg to extract frames, and gifski to encode GIFs. The browser UI runs on a Rust/axum backend with a VideoJS player.

## Requirements

- A current stable [Rust toolchain](https://www.rust-lang.org/tools/install) with Cargo.
- `yt-dlp`, `ffmpeg`, and `gifski` available on your `PATH`.

On macOS, install the video tools with Homebrew:

```sh
brew install yt-dlp ffmpeg gifski
```

On other platforms, install the same tools using your package manager or their upstream installation instructions.

## Install

```sh
git clone https://github.com/bradleydwyer/snag.git
cd snag
cargo install --path . --locked
```

Make sure Cargo's bin directory (usually `~/.cargo/bin`) is on your `PATH`.

## Browser UI

```sh
snag
```

This starts the server and opens `http://localhost:8080` in your browser. If the browser opens before the server is ready, refresh the page.

1. Paste a YouTube URL and click **Load**. Wait for the video to download.
2. Seek to the start of your clip and click **Mark In**, then seek to the end and click **Mark Out**.
3. Fine-tune the selection by dragging the timeline handles, editing the time fields (`MM:SS.ss`), or using the step buttons.
4. Click **Encode GIF**, preview the result, and click **Download** to save it.

Keyboard shortcuts work when you're not typing in an input:

| Key | Action |
| --- | --- |
| `i` | Mark the current position as the start |
| `o` | Mark the current position as the end |
| `Enter` | Encode the selected clip |

To change the port or GIF settings:

```sh
snag --port 9090 --width 640 --fps 20
```

Videos and GIFs created by the browser UI are stored in temporary directories for the server session. Download your GIF before stopping the server with `Ctrl+C`. The player loads its JavaScript from jsDelivr, so the browser needs internet access.

## Direct encoding

Provide a URL and both `--start` and `--end` to encode without opening the browser:

```sh
snag 'https://www.youtube.com/watch?v=VIDEO_ID' \
  --start 10 \
  --end 13.5 \
  --width 480 \
  --fps 15 \
  --output clip.gif
```

Times are in seconds and can include decimals. The start must be earlier than the end. If you omit `--output`, the GIF is written to `output.gif` in the current directory.

## Options

| Option | Description | Default |
| --- | --- | --- |
| `[URL]` | YouTube URL; required for direct encoding | |
| `-s, --start <SECONDS>` | Start time for direct encoding | |
| `-e, --end <SECONDS>` | End time for direct encoding | |
| `-o, --output <PATH>` | Output path for direct encoding | `output.gif` |
| `-W, --width <PIXELS>` | GIF width; height keeps the video's aspect ratio | `480` |
| `--fps <FPS>` | GIF frame rate | `15` |
| `-p, --port <PORT>` | Browser UI server port | `8080` |
| `-h, --help` | Show command-line help | |

Without the URL and both time options, snag starts the browser UI; paste the URL into the page to load a video.

## Development

Run from the checkout without installing:

```sh
cargo run --locked
```

Pass options after `--`, for example:

```sh
cargo run --locked -- --port 9090
```
