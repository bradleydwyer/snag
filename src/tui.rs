use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use ratatui_image::StatefulImage;
use std::io;

use crate::player::{Player, format_time};

pub enum TuiResult {
    Encode,
    Quit,
}

pub fn run(player: &mut Player) -> Result<TuiResult> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut picker = Picker::from_query_stdio()?;

    let mut protocol_cache: Vec<Option<StatefulProtocol>> =
        (0..player.frame_count()).map(|_| None).collect();

    let result = run_loop(&mut terminal, &mut picker, player, &mut protocol_cache);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    picker: &mut Picker,
    player: &mut Player,
    cache: &mut [Option<StatefulProtocol>],
) -> Result<TuiResult> {
    loop {
        let cursor = player.cursor;

        if cache[cursor].is_none() {
            cache[cursor] = Some(picker.new_resize_protocol(player.frames[cursor].clone()));
        }

        terminal.draw(|f| {
            let chunks = Layout::vertical([
                Constraint::Min(8),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(f.area());

            draw_frame(f, chunks[0], cache, cursor);
            draw_timeline(f, chunks[1], player);
            draw_help(f, chunks[2], player);
        })?;

        if let Event::Key(key) = event::read()? {
            match handle_key(key, player) {
                Some(TuiResult::Encode) => return Ok(TuiResult::Encode),
                Some(TuiResult::Quit) => return Ok(TuiResult::Quit),
                None => {}
            }
        }
    }
}

fn handle_key(key: KeyEvent, player: &mut Player) -> Option<TuiResult> {
    // Jump sizes in frames:
    // h/l = 1 frame, H/L or Shift+arrows = 1 second worth, Ctrl+arrows = 5 seconds worth
    let small_jump = player.preview_fps.round() as isize; // ~1 second
    let big_jump = (player.preview_fps * 5.0).round() as isize; // ~5 seconds

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(TuiResult::Quit),
        KeyCode::Enter => {
            if player.has_selection() {
                Some(TuiResult::Encode)
            } else {
                None
            }
        }
        // Big jump: Ctrl+arrow or J/K
        KeyCode::Left if key.modifiers.contains(KeyModifiers::CONTROL) => {
            player.move_cursor(-big_jump);
            None
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::CONTROL) => {
            player.move_cursor(big_jump);
            None
        }
        KeyCode::Char('J') => {
            player.move_cursor(-big_jump);
            None
        }
        KeyCode::Char('K') => {
            player.move_cursor(big_jump);
            None
        }
        // Medium jump (~1s): Shift+arrow or H/L
        KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
            player.move_cursor(-small_jump);
            None
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
            player.move_cursor(small_jump);
            None
        }
        KeyCode::Char('H') => {
            player.move_cursor(-small_jump);
            None
        }
        KeyCode::Char('L') => {
            player.move_cursor(small_jump);
            None
        }
        // Single frame: h/l or arrow keys
        KeyCode::Char('h') | KeyCode::Left => {
            player.move_cursor(-1);
            None
        }
        KeyCode::Char('l') | KeyCode::Right => {
            player.move_cursor(1);
            None
        }
        KeyCode::Char('a') => {
            player.set_start();
            None
        }
        KeyCode::Char('b') => {
            player.set_end();
            None
        }
        KeyCode::Char('r') => {
            player.reset();
            None
        }
        _ => None,
    }
}

fn draw_frame(
    f: &mut ratatui::Frame,
    area: Rect,
    cache: &mut [Option<StatefulProtocol>],
    cursor: usize,
) {
    let block = Block::default().borders(Borders::ALL).title(" Preview ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(ref mut proto) = cache[cursor] {
        let image = StatefulImage::default();
        f.render_stateful_widget(image, inner, proto);
    }
}

fn draw_timeline(f: &mut ratatui::Frame, area: Rect, player: &Player) {
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 4 {
        return;
    }

    let bar_width = inner.width as usize;
    let total = player.frame_count().max(1);

    let cursor_pos = (player.cursor * bar_width) / total;
    let start_pos = player.start.map(|s| (s * bar_width) / total);
    let end_pos = player.end.map(|e| (e * bar_width) / total);

    let mut bar: Vec<Span> = Vec::new();
    for i in 0..bar_width {
        let is_cursor = i == cursor_pos;
        let is_start = start_pos.is_some_and(|s| i == s);
        let is_end = end_pos.is_some_and(|e| i == e);
        let in_range = match (start_pos, end_pos) {
            (Some(s), Some(e)) => i >= s && i <= e,
            _ => false,
        };

        if is_cursor {
            bar.push(Span::styled("▊", Style::default().fg(Color::Yellow).bold()));
        } else if is_start {
            bar.push(Span::styled("A", Style::default().fg(Color::Green).bold()));
        } else if is_end {
            bar.push(Span::styled("B", Style::default().fg(Color::Red).bold()));
        } else if in_range {
            bar.push(Span::styled("━", Style::default().fg(Color::Cyan)));
        } else {
            bar.push(Span::styled("─", Style::default().fg(Color::DarkGray)));
        }
    }

    let time_info = format!(
        " {} / {}  ",
        format_time(player.cursor_time()),
        format_time(player.duration_secs)
    );

    let selection_info = match (player.start_time(), player.end_time()) {
        (Some(s), Some(e)) => format!("[{} → {} ({:.1}s)]", format_time(s), format_time(e), e - s),
        (Some(s), None) => format!("[A: {}]", format_time(s)),
        (None, Some(e)) => format!("[B: {}]", format_time(e)),
        _ => String::new(),
    };

    bar.push(Span::raw(time_info));
    bar.push(Span::styled(selection_info, Style::default().fg(Color::Cyan)));

    f.render_widget(Paragraph::new(Line::from(bar)), inner);
}

fn draw_help(f: &mut ratatui::Frame, area: Rect, player: &Player) {
    let encode_hint = if player.has_selection() {
        Span::styled("Enter: encode  ", Style::default().fg(Color::Green))
    } else {
        Span::styled("Enter: encode  ", Style::default().fg(Color::DarkGray))
    };

    let help = Line::from(vec![
        Span::raw(" h/l: frame  H/L: 1s  J/K: 5s  "),
        Span::styled("a", Style::default().fg(Color::Green)),
        Span::raw(": start  "),
        Span::styled("b", Style::default().fg(Color::Red)),
        Span::raw(": end  "),
        Span::raw("r: reset  "),
        encode_hint,
        Span::raw("q: quit"),
    ]);

    f.render_widget(Paragraph::new(help), area);
}
