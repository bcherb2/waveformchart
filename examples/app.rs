use std::collections::VecDeque;
use std::time::{Duration, Instant};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use waveformchart::{WaveformMode, WaveformWidget};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataSource {
    Cpu,
    Memory,
}

impl DataSource {
    fn next(&self) -> Self {
        match self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Cpu,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "MEM",
        }
    }
}

pub struct App {
    // System monitoring
    sys: System,
    cpu_history: VecDeque<f64>,
    mem_history: VecDeque<f64>,
    max_history: usize,

    // Configuration
    pub running: bool,
    pub tick_rate: Duration,
    pub last_tick: Instant,
    
    // Widget State
    pub top_source: DataSource,
    pub bottom_source: DataSource,
    pub mode: WaveformMode,
    pub fade_effect: bool,
    pub gradient_effect: bool,
    pub autoscale: bool,
    
    // Visuals
    pub top_color_idx: usize,
    pub bottom_color_idx: usize,
    pub colors: Vec<Color>,
}

impl App {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        // Initial refresh to get data
        sys.refresh_cpu_all();
        sys.refresh_memory();

        Self {
            sys,
            cpu_history: VecDeque::with_capacity(500),
            mem_history: VecDeque::with_capacity(500),
            max_history: 500, // Store enough for wide screens

            running: true,
            tick_rate: Duration::from_millis(100),
            last_tick: Instant::now(),

            top_source: DataSource::Cpu,
            bottom_source: DataSource::Memory,
            mode: WaveformMode::HighResBraille,
            fade_effect: false,
            gradient_effect: false,
            autoscale: false, // Default to Fixed 100%

            top_color_idx: 2, // Green
            bottom_color_idx: 4, // Blue
            colors: vec![
                Color::Reset,
                Color::Red,
                Color::Green,
                Color::Yellow,
                Color::Blue,
                Color::Magenta,
                Color::Cyan,
                Color::White,
            ],
        }
    }

    pub fn on_tick(&mut self) {
        // Refresh system stats
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();

        // Collect CPU (global usage)
        let cpu_usage = self.sys.global_cpu_usage() as f64 / 100.0;
        Self::push_history(&mut self.cpu_history, cpu_usage, self.max_history);

        // Collect Memory with simulated noise for demo purposes
        let total_mem = self.sys.total_memory() as f64;
        let used_mem = self.sys.used_memory() as f64;
        let mut mem_usage = if total_mem > 0.0 { used_mem / total_mem } else { 0.0 };
        
        // Add some random noise (-2% to +2%) to make the chart look alive
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let noise: f64 = rng.gen_range(-0.02..0.02);
        mem_usage = (mem_usage + noise).clamp(0.0, 1.0);

        Self::push_history(&mut self.mem_history, mem_usage, self.max_history);
    }

    fn push_history(history: &mut VecDeque<f64>, value: f64, max_history: usize) {
        if history.len() >= max_history {
            history.pop_front();
        }
        history.push_back(value);
    }

    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => self.running = false,
                    KeyCode::Char('1') => self.top_source = self.top_source.next(),
                    KeyCode::Char('2') => self.bottom_source = self.bottom_source.next(),
                    KeyCode::Char('+') => {
                        let new_millis = self.tick_rate.as_millis().saturating_sub(10);
                        if new_millis > 0 {
                            self.tick_rate = Duration::from_millis(new_millis as u64);
                        }
                    }
                    KeyCode::Char('-') => {
                        let new_millis = self.tick_rate.as_millis().saturating_add(10);
                        self.tick_rate = Duration::from_millis(new_millis as u64);
                    }
                    KeyCode::Char('c') => {
                        self.top_color_idx = (self.top_color_idx + 1) % self.colors.len();
                        self.bottom_color_idx = (self.bottom_color_idx + 1) % self.colors.len();
                    }
                    KeyCode::Char('m') => {
                        self.mode = match self.mode {
                            WaveformMode::HighResBraille => WaveformMode::UltraThinBlock,
                            WaveformMode::UltraThinBlock => WaveformMode::HighResBraille,
                        };
                    }
                    KeyCode::Char('f') => {
                        self.fade_effect = !self.fade_effect;
                    }
                    KeyCode::Char('g') => {
                        self.gradient_effect = !self.gradient_effect;
                    }
                    KeyCode::Char('s') => {
                        self.autoscale = !self.autoscale;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // Status bar
            ])
            .split(f.area());

        let main_area = chunks[0];
        let status_area = chunks[1];

        // Prepare data slices based on width
        let width = main_area.width as usize;

        // Ensure contiguousness upfront for both, then borrow slices.
        self.cpu_history.make_contiguous();
        self.mem_history.make_contiguous();
        
        let top_data = match self.top_source {
            DataSource::Cpu => self.cpu_history.as_slices().0,
            DataSource::Memory => self.mem_history.as_slices().0,
        };
        
        let bottom_data = match self.bottom_source {
            DataSource::Cpu => self.cpu_history.as_slices().0,
            DataSource::Memory => self.mem_history.as_slices().0,
        };
        
        // Slice to width
        let top_len = top_data.len();
        let top_start = top_len.saturating_sub(width);
        let top_data = &top_data[top_start..];
        
        let bottom_len = bottom_data.len();
        let bottom_start = bottom_len.saturating_sub(width);
        let bottom_data = &bottom_data[bottom_start..];

        let top_color = self.colors[self.top_color_idx];
        let bottom_color = self.colors[self.bottom_color_idx];

        // Calculate max values if autoscaling
        let top_max = if self.autoscale {
            top_data.iter().fold(0.0f64, |a, &b| a.max(b)).max(0.001) // Avoid div by zero
        } else {
            1.0
        };

        let bottom_max = if self.autoscale {
            bottom_data.iter().fold(0.0f64, |a, &b| a.max(b)).max(0.001)
        } else {
            1.0
        };

        let widget = WaveformWidget::new(top_data, bottom_data)
            .block(Block::default().borders(Borders::ALL).title(" System Monitor "))
            .mode(self.mode)
            .fade_effect(self.fade_effect)
            .gradient_effect(self.gradient_effect)
            .top_style(Style::default().fg(top_color))
            .bottom_style(Style::default().fg(bottom_color))
            .top_max(top_max)
            .bottom_max(bottom_max);

        f.render_widget(widget, main_area);

        // Status Bar
        let status_text = vec![
            Span::raw(" [q] Quit "),
            Span::raw(" [1] Top: "),
            Span::styled(self.top_source.label(), Style::default().fg(top_color).add_modifier(Modifier::BOLD)),
            Span::raw(" [2] Bot: "),
            Span::styled(self.bottom_source.label(), Style::default().fg(bottom_color).add_modifier(Modifier::BOLD)),
            Span::raw(format!(" [+/-] Speed: {}ms ", self.tick_rate.as_millis())),
            Span::raw(" [c] Color "),
            Span::raw(" [m] Mode "),
            Span::raw(if self.fade_effect { " [f] Fade: ON " } else { " [f] Fade: OFF " }),
            Span::raw(if self.fade_effect { " [f] Fade: ON " } else { " [f] Fade: OFF " }),
            Span::raw(if self.gradient_effect { " [g] Grad: ON " } else { " [g] Grad: OFF " }),
            Span::raw(if self.autoscale { " [s] Scale: AUTO " } else { " [s] Scale: 100% " }),
        ];
        
        let status_paragraph = Paragraph::new(Line::from(status_text))
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        
        f.render_widget(status_paragraph, status_area);
    }


}
