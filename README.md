# Waveform Chart Widget for Ratatui

A high-performance, high-resolution waveform chart widget for [Ratatui](https://github.com/ratatui-org/ratatui). Designed for audio visualization, system monitoring, and other real-time data feeds.

## Features

*   **High Resolution:** Uses Braille characters (`⠀` to `⣿`) to achieve **4x vertical resolution** per terminal cell.
*   **Dual Channel:** Renders two data series simultaneously (Top and Bottom) mirroring each other, perfect for stereo audio or input/output monitoring.
*   **Advanced Visual Effects:**
    *   **Horizontal Fade:** Smoothly dims older data points to visualize time progression (Linear fade with delayed start).
    *   **Vertical Gradient:** Modulates brightness based on signal height (Center is bright, peaks fade out).
*   **Flexible Scaling:** Supports both **Fixed** (0-100%) and **Autoscaling** modes.
*   **Customizable:** Full control over colors, styles, and rendering modes (Braille vs. Block).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
waveformchart = { git = "https://github.com/bcherb2/waveformchart" }
ratatui = "0.29"
```

## Usage

```rust
use waveformchart::{WaveformWidget, WaveformMode};
use ratatui::{prelude::*, widgets::*};

fn render(frame: &mut Frame, area: Rect) {
    // Data should be normalized (0.0 to 1.0)
    let top_data = vec![0.0, 0.2, 0.5, 0.8, 1.0];
    let bottom_data = vec![0.1, 0.3, 0.4, 0.7, 0.9];

    let widget = WaveformWidget::new(&top_data, &bottom_data)
        .mode(WaveformMode::HighResBraille)
        .top_style(Style::default().fg(Color::Cyan))
        .bottom_style(Style::default().fg(Color::Magenta))
        .fade_effect(true)
        .gradient_effect(true)
        .top_max(1.0) // Fixed scale
        .bottom_max(1.0);

    frame.render_widget(widget, area);
}
```

## Running the Demo

Clone the repository and run the example:

```bash
cargo run --example demo
```

### Controls

| Key | Action |
| :--- | :--- |
| `q` | Quit |
| `m` | Toggle Mode (Braille / Block) |
| `f` | Toggle Horizontal Fade |
| `g` | Toggle Vertical Gradient |
| `s` | Toggle Autoscale (Fixed 100% vs Auto) |
| `c` | Cycle Colors |
| `1` | Toggle Top Source (CPU / Memory) |
| `2` | Toggle Bottom Source (CPU / Memory) |
| `+`/`-` | Adjust Speed |

## License

MIT
