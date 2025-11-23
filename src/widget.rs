use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};

/// Defines the rendering style of the waveform columns.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WaveformMode {
    /// High vertical resolution (4x) using only the left column of Braille dots.
    /// Visually thin dots, smooth peaks/valleys.
    /// Uses: ⡀ ⡄ ⡆ ⡇
    #[default]
    HighResBraille,

    /// Standard vertical resolution (1x) using the Left Half Block character.
    /// Visually solid blocks, "steppy" vertical changes.
    /// Uses: ▌
    UltraThinBlock,
}

/// A Ratatui widget for rendering high-resolution waveform charts.
///
/// The `WaveformWidget` supports two modes:
/// - `HighResBraille`: Uses Braille characters (4x2 dots) for 4x vertical resolution per cell.
/// - `UltraThinBlock`: Uses thin block characters for a cleaner, blocky look.
///
/// It also supports advanced visual effects:
/// - **Horizontal Fade**: Dims older data points (left side) to visualize time progression.
/// - **Vertical Gradient**: Changes color brightness based on signal height (peaks are dimmer).
///
/// # Example
/// ```rust
/// use waveformchart::{WaveformWidget, WaveformMode};
/// use ratatui::style::{Style, Color};
///
/// let top_data = vec![0.1, 0.5, 0.8, 0.3];
/// let bottom_data = vec![0.2, 0.4, 0.6, 0.1];
///
/// let widget = WaveformWidget::new(&top_data, &bottom_data)
///     .mode(WaveformMode::HighResBraille)
///     .top_style(Style::default().fg(Color::Green))
///     .bottom_style(Style::default().fg(Color::Blue));
/// ```
pub struct WaveformWidget<'a> {
    /// Optional surrounding block (borders, titles).
    block: Option<Block<'a>>,

    /// The active rendering mode.
    mode: WaveformMode,

    /// Normalized data (0.0 - 1.0) rendered upwards from center.
    top_data: &'a [f64],
    top_style: Style,

    /// Normalized data (0.0 - 1.0) rendered downwards from center.
    bottom_data: &'a [f64],
    bottom_style: Style,

    /// If true, applies a horizontal fade effect (dimming older data).
    fade_effect: bool,
    
    /// If true, applies a vertical gradient effect (color changes with height).
    gradient_effect: bool,

    /// Maximum value for scaling (default 1.0)
    top_max: f64,
    bottom_max: f64,
}

impl<'a> WaveformWidget<'a> {
    /// Creates a new widget with required data references.
    /// Data must be normalized between 0.0 and 1.0.
    pub fn new(top_data: &'a [f64], bottom_data: &'a [f64]) -> Self {
        Self {
            top_data,
            bottom_data,
            block: None,
            mode: WaveformMode::HighResBraille,
            fade_effect: false,
            gradient_effect: false,
            top_style: Style::default(),
            bottom_style: Style::default(),
            top_max: 1.0,
            bottom_max: 1.0,
        }
    }

    /// Sets an optional surrounding block.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the rendering mode.
    pub fn mode(mut self, mode: WaveformMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the style (color, modifier) for the top half.
    pub fn top_style(mut self, style: Style) -> Self {
        self.top_style = style;
        self
    }

    /// Sets the style (color, modifier) for the bottom half.
    pub fn bottom_style(mut self, style: Style) -> Self {
        self.bottom_style = style;
        self
    }

    /// Enables or disables the horizontal fade effect.
    pub fn fade_effect(mut self, enable: bool) -> Self {
        self.fade_effect = enable;
        self
    }

    /// Enables or disables the vertical gradient effect.
    pub fn gradient_effect(mut self, enable: bool) -> Self {
        self.gradient_effect = enable;
        self
    }

    pub fn top_max(mut self, max: f64) -> Self {
        self.top_max = max;
        self
    }

    pub fn bottom_max(mut self, max: f64) -> Self {
        self.bottom_max = max;
        self
    }
}

impl<'a> Widget for WaveformWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = match &self.block {
            Some(b) => {
                let inner = b.inner(area);
                b.render(area, buf);
                inner
            }
            None => area,
        };

        if inner_area.height < 1 || inner_area.width < 1 {
            return;
        }

        let center_y = inner_area.top() + (inner_area.height / 2);
        let max_char_height = inner_area.height / 2;
        
        let data_len = self.top_data.len().min(self.bottom_data.len());
        let width = inner_area.width as usize;
        let start_x_offset = width.saturating_sub(data_len) as u16;

        for x in inner_area.left()..inner_area.right() {
            let relative_x = x - inner_area.left();
            
            if relative_x < start_x_offset {
                continue;
            }
            
            let data_index = (relative_x - start_x_offset) as usize;
            
            // Bounds check
            if data_index >= self.top_data.len() || data_index >= self.bottom_data.len() {
                continue;
            }

            // Normalize data based on max value (default 1.0)
            let top_val = (self.top_data[data_index] / self.top_max).clamp(0.0, 1.0);
            let bottom_val = (self.bottom_data[data_index] / self.bottom_max).clamp(0.0, 1.0);

            // Calculate fade factor
            let fade_factor = if self.fade_effect {
                let relative_x_f = (x - inner_area.left()) as f64;
                let width_f = inner_area.width as f64;
                // 0.0 (left) to 1.0 (right)
                // We want right to be 1.0 (bright), left to be 0.0 (invisible)
                // Using a power curve makes the fade more dramatic
                let linear = relative_x_f / width_f;
                // Delayed fade: Right half (0.5-1.0) is full brightness
                // Left half (0.0-0.5) fades linearly from 0.0 to 1.0
                if linear > 0.5 {
                    1.0
                } else {
                    linear * 2.0
                }
            } else {
                1.0
            };

            // Base styles (no fade yet)
            let top_base_style = self.top_style;
            let bottom_base_style = self.bottom_style;

            match self.mode {
                WaveformMode::HighResBraille => {
                    self.render_braille_column(buf, x, center_y, max_char_height, top_val, true, top_base_style, self.gradient_effect, fade_factor);
                    self.render_braille_column(buf, x, center_y, max_char_height, bottom_val, false, bottom_base_style, self.gradient_effect, fade_factor);
                }
                WaveformMode::UltraThinBlock => {
                    self.render_block_column(buf, x, center_y, max_char_height, top_val, true, inner_area, top_base_style, self.gradient_effect, fade_factor);
                    self.render_block_column(buf, x, center_y, max_char_height, bottom_val, false, inner_area, bottom_base_style, self.gradient_effect, fade_factor);
                }
            }
        }
    }
}

fn apply_fade(mut style: Style, factor: f64) -> Style {
    // Removed early return to ensure consistent RGB conversion
    // even when factor is 1.0. This prevents "Named Color" vs "RGB Color" mismatches.
    
    // Apply DIM modifier for extra fading hint
    // Removed DIM modifier as it might cause desaturation on some terminals
    // if factor < 0.5 {
    //    style = style.add_modifier(ratatui::style::Modifier::DIM);
    // }
    
    let (r, g, b) = match style.fg {
        Some(c) => color_to_rgb(c),
        None => return style,
    };

    let new_r = (r as f64 * factor) as u8;
    let new_g = (g as f64 * factor) as u8;
    let new_b = (b as f64 * factor) as u8;

    style.fg(Color::Rgb(new_r, new_g, new_b))
}

fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(i) => {
             match i {
                 // Standard 16 colors approximation
                 0 => (0, 0, 0), // Black
                 1 => (170, 0, 0), // Red
                 2 => (0, 170, 0), // Green
                 3 => (170, 85, 0), // Yellow
                 4 => (0, 0, 170), // Blue
                 5 => (170, 0, 170), // Magenta
                 6 => (0, 170, 170), // Cyan
                 7 => (170, 170, 170), // Gray
                 8 => (85, 85, 85), // DarkGray
                 9 => (255, 85, 85), // LightRed
                 10 => (85, 255, 85), // LightGreen
                 11 => (255, 255, 85), // LightYellow
                 12 => (85, 85, 255), // LightBlue
                 13 => (255, 85, 255), // LightMagenta
                 14 => (85, 255, 255), // LightCyan
                 15 => (255, 255, 255), // White
                 _ => (255, 255, 255), // Default to white for unknown
             }
        },
        Color::Black => (0, 0, 0),
        Color::Red => (170, 0, 0),
        Color::Green => (0, 170, 0),
        Color::Yellow => (170, 85, 0),
        Color::Blue => (0, 0, 170),
        Color::Magenta => (170, 0, 170),
        Color::Cyan => (0, 170, 170),
        Color::Gray => (170, 170, 170),
        Color::DarkGray => (85, 85, 85),
        Color::LightRed => (255, 85, 85),
        Color::LightGreen => (85, 255, 85),
        Color::LightYellow => (255, 255, 85),
        Color::LightBlue => (85, 85, 255),
        Color::LightMagenta => (255, 85, 255),
        Color::LightCyan => (85, 255, 255),
        Color::White => (255, 255, 255),
        _ => (255, 255, 255),
    }
}

impl<'a> WaveformWidget<'a> {
    fn render_braille_column(
        &self,
        buf: &mut Buffer,
        x: u16,
        center_y: u16,
        max_char_height: u16,
        val: f64,
        is_top: bool,
        base_style: Style,
        use_gradient: bool,
        fade_factor: f64,
    ) {
        let total_dots = max_char_height as f64 * 4.0;
        let needed_dots = (val * total_dots).round() as u16;
        
        let mut dots_remaining = needed_dots;
        let mut y = if is_top { center_y.saturating_sub(1) } else { center_y };

        for i in 0..max_char_height {
            if dots_remaining == 0 {
                break;
            }

            let char_to_draw = if dots_remaining >= 4 {
                dots_remaining -= 4;
                '\u{2847}' // Full height ⡇
            } else {
                let c = if is_top {
                    get_thin_braille_fill(dots_remaining as u8)
                } else {
                    get_thin_braille_fill_bottom(dots_remaining as u8)
                };
                dots_remaining = 0;
                c
            };
            
            let style = if use_gradient {
                // Calculate height ratio (0.0 at center, 1.0 at peak)
                let height_ratio = i as f64 / max_char_height as f64;
                apply_gradient(base_style, height_ratio)
            } else {
                base_style
            };
            
            // Apply fade LAST so it dims whatever color we have
            let final_style = apply_fade(style, fade_factor);

            buf[(x, y)].set_char(char_to_draw).set_style(final_style);

            if is_top {
                if y == 0 { break; } // Prevent underflow
                y -= 1;
            } else {
                y += 1;
            }
        }
    }

    fn render_block_column(
        &self,
        buf: &mut Buffer,
        x: u16,
        center_y: u16,
        max_char_height: u16,
        val: f64,
        is_top: bool,
        inner_area: Rect,
        base_style: Style,
        use_gradient: bool,
        fade_factor: f64,
    ) {
        let needed_rows = (val * max_char_height as f64).round() as u16;

        for i in 0..needed_rows {
             let y = if is_top {
                (center_y.saturating_sub(1)).saturating_sub(i)
            } else {
                center_y + i
            };

            // Bounds check
            if is_top {
                if y < inner_area.top() { continue; }
            } else {
                if y >= inner_area.bottom() { continue; }
            }
            
            let style = if use_gradient {
                let height_ratio = i as f64 / max_char_height as f64;
                apply_gradient(base_style, height_ratio)
            } else {
                base_style
            };

            // Apply fade LAST
            let final_style = apply_fade(style, fade_factor);

            buf[(x, y)].set_char('▌').set_style(final_style);
        }
    }
}

fn apply_gradient(style: Style, ratio: f64) -> Style {
    // Inverted Gradient:
    // Center (ratio 0.0) = Full Brightness (1.0)
    // Peak (ratio 1.0) = Dimmer (e.g. 30% brightness)
    
    if let Some(color) = style.fg {
        let (r, g, b) = color_to_rgb(color);

        // Brightness decreases as we go away from center
        let brightness = 1.0 - (ratio * 0.7);
        
        let new_r = (r as f64 * brightness) as u8;
        let new_g = (g as f64 * brightness) as u8;
        let new_b = (b as f64 * brightness) as u8;
        
        style.fg(Color::Rgb(new_r, new_g, new_b))
    } else {
        style
    }
}

// This function must only be called when mode is HighResBraille.
// height_in_dots must be between 1 and 4 inclusive.
fn get_thin_braille_fill(height_in_dots: u8) -> char {
    match height_in_dots {
        // Dot 7 only
        1 => '\u{2840}', // ⡀
        // Dots 7 and 3
        2 => '\u{2844}', // ⡄
        // Dots 7, 3, and 2
        3 => '\u{2846}', // ⡆
        // Dots 7, 3, 2, and 1 (Full left column)
        4 => '\u{2847}', // ⡇
        // Fallback for safety
        _ => ' ',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_thin_braille_fill() {
        assert_eq!(get_thin_braille_fill(1), '\u{2840}');
        assert_eq!(get_thin_braille_fill(2), '\u{2844}');
        assert_eq!(get_thin_braille_fill(3), '\u{2846}');
        assert_eq!(get_thin_braille_fill(4), '\u{2847}');
        assert_eq!(get_thin_braille_fill(0), ' ');
        assert_eq!(get_thin_braille_fill(5), ' ');
    }

    #[test]
    fn test_get_thin_braille_fill_bottom() {
        assert_eq!(get_thin_braille_fill_bottom(1), '\u{2801}');
        assert_eq!(get_thin_braille_fill_bottom(2), '\u{2803}');
        assert_eq!(get_thin_braille_fill_bottom(3), '\u{2807}');
        assert_eq!(get_thin_braille_fill_bottom(4), '\u{2847}');
        assert_eq!(get_thin_braille_fill_bottom(0), ' ');
        assert_eq!(get_thin_braille_fill_bottom(5), ' ');
    }

    #[test]
    fn test_apply_fade() {
        let style = Style::default().fg(Color::Rgb(100, 200, 50));
        
        // 100% factor -> Same color
        let faded_100 = apply_fade(style, 1.0);
        assert_eq!(faded_100.fg, Some(Color::Rgb(100, 200, 50)));

        // 50% factor -> Half brightness
        let faded_50 = apply_fade(style, 0.5);
        assert_eq!(faded_50.fg, Some(Color::Rgb(50, 100, 25)));

        // 0% factor -> Black
        let faded_0 = apply_fade(style, 0.0);
        assert_eq!(faded_0.fg, Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn test_apply_gradient() {
        let style = Style::default().fg(Color::Rgb(0, 0, 255)); // Blue
        
        // 0% ratio (Center) -> Full Brightness
        // B: 255 * 1.0 = 255
        let grad_0 = apply_gradient(style, 0.0);
        assert_eq!(grad_0.fg, Some(Color::Rgb(0, 0, 255)));

        // 100% ratio (Peak) -> Dimmer (30% brightness)
        // B: 255 * 0.3 = 76.5 -> 76
        let grad_100 = apply_gradient(style, 1.0);
        assert_eq!(grad_100.fg, Some(Color::Rgb(0, 0, 76)));
    }
}

// This function must only be called when mode is HighResBraille.
// height_in_dots must be between 1 and 4 inclusive.
// Returns characters with dots aligned to the TOP of the cell (for growing downwards).
fn get_thin_braille_fill_bottom(height_in_dots: u8) -> char {
    match height_in_dots {
        // Dot 1 only (Top Left)
        1 => '\u{2801}', // ⠁
        // Dots 1 and 2 (Top two)
        2 => '\u{2803}', // ⠃
        // Dots 1, 2, and 3 (Top three)
        3 => '\u{2807}', // ⠇
        // Dots 1, 2, 3, and 7 (Full left column)
        4 => '\u{2847}', // ⡇
        // Fallback for safety
        _ => ' ',
    }
}
