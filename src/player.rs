use image::DynamicImage;

pub struct Player {
    pub frames: Vec<DynamicImage>,
    pub cursor: usize,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub duration_secs: f64,
    pub preview_fps: f64,
}

impl Player {
    pub fn new(frames: Vec<DynamicImage>, duration_secs: f64, preview_fps: f64) -> Self {
        Self {
            frames,
            cursor: 0,
            start: None,
            end: None,
            duration_secs,
            preview_fps,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn cursor_time(&self) -> f64 {
        self.cursor as f64 / self.preview_fps
    }

    pub fn start_time(&self) -> Option<f64> {
        self.start.map(|i| i as f64 / self.preview_fps)
    }

    pub fn end_time(&self) -> Option<f64> {
        self.end.map(|i| i as f64 / self.preview_fps)
    }

    pub fn set_start(&mut self) {
        self.start = Some(self.cursor);
        if let Some(end) = self.end {
            if end <= self.cursor {
                self.end = None;
            }
        }
    }

    pub fn set_end(&mut self) {
        self.end = Some(self.cursor);
        if let Some(start) = self.start {
            if start >= self.cursor {
                self.start = None;
            }
        }
    }

    pub fn reset(&mut self) {
        self.start = None;
        self.end = None;
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let new = self.cursor as isize + delta;
        self.cursor = new.clamp(0, (self.frame_count() - 1) as isize) as usize;
    }

    pub fn has_selection(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }
}

pub fn format_time(secs: f64) -> String {
    let m = (secs / 60.0) as u64;
    let s = secs % 60.0;
    format!("{m}:{s:05.2}")
}
