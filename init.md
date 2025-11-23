Here is the complete technical specification document for the custom Ratatui widget, incorporating the corrected Unicode Braille characters for the single-column aesthetic.

-----

# Technical Specification: Ratatui `WaveformWidget`

## 1\. Overview

The `WaveformWidget` is a custom rendering component for the Ratatui TUI library. It is designed to visualize two related data series simultaneously as a mirrored "digital audio waveform."

The widget establishes a horizontal centerline within its allocated area. Data Series A (e.g., CPU load) is rendered growing upwards from this line, while Data Series B (e.g., Memory usage) grows downwards from the same line.

A key feature of this widget is its ability to render thin, distinct vertical columns for every horizontal tick, avoiding the "blocky" look of standard character-cell bar charts.

## 2\. Rendering Modes

The widget supports two distinct rendering modes to balance vertical resolution against visual style.

### Mode A: `HighResBraille` (Default)

This mode utilizes specific Unicode Braille characters to achieve **4x the vertical resolution** of a standard terminal row.

Crucially, to maintain a thin, waveform-like aesthetic, it uses **only the left column of dots** (dots 1, 2, 3, and 7) within the 2x4 Braille cell grid. This ensures every horizontal data tick is rendered as a single, precise vertical line of dots.

**Characters used:**

  * 1/4 height: `⡀` (U+2840)
  * 2/4 height: `⡄` (U+2844)
  * 3/4 height: `⡆` (U+2846)
  * Full height: `⡇` (U+2847)

### Mode B: `UltraThinBlock`

This mode provides a "chunkier," more retro aesthetic with **1x vertical resolution** per terminal row.

It uses the Unicode "Left Half Block" character. While visually slightly wider than the single-dot column of Braille, it is the thinnest possible solid block character in TUIs.

**Character used:**

  * Full height: `▌` (U+258C)

-----

## 3\. API Surface

### 3.1 Data Structures

```rust
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
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

/// The main widget structure.
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
}
```

### 3.2 Constructor and Builder Methods

The widget should adhere to standard Ratatui builder patterns.

```rust
impl<'a> WaveformWidget<'a> {
    /// Creates a new widget with required data references.
    /// Data must be normalized between 0.0 and 1.0.
    pub fn new(top_data: &'a [f64], bottom_data: &'a [f64]) -> Self {
        Self {
            block: None,
            mode: WaveformMode::default(),
            top_data,
            top_style: Style::default(),
            bottom_data,
            bottom_style: Style::default(),
            fade_effect: false,
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
}
```

-----

## 4\. Internal Logic (Helpers)

To support `WaveformMode::HighResBraille`, a helper function is required to map a desired dot height (1-4) to the correct single-column Braille character.

```rust
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
        // Fallback for safety, though technically unreachable if logic is correct
        _ => ' ',
    }
}
```

-----

## 5\. Rendering Algorithm (Implementation Guide)

Implement `impl<'a> Widget for WaveformWidget<'a>` with the following logic inside the `render` function.

### Phase 1: Setup and Layout

1.  **Block Rendering:** If `self.block` is defined, render it into the provided `area`. Create a new `inner_area` set to `block.inner(area)`. If no block, `inner_area` is equal to `area`.
2.  **Centerline Calculation:** Determine the Y coordinate of the horizontal center.
    ```rust
    let center_y = inner_area.top() + (inner_area.height() / 2);
    ```
3.  **Max Height Calculation:** Determine the maximum height available in character rows for one half of the waveform.
    ```rust
    let max_char_height = inner_area.height() / 2;
    ```

### Phase 2: The Main Drawing Loop

Iterate horizontally through the columns of the `inner_area`.

For each column `x` from `inner_area.left()` to `inner_area.right()`:

1.  **Data Mapping:** Map the current screen `x` coordinate to an index in the data slices. Ensure bounds checking so you don't panic if the area is wider than the data slice length.
2.  **Retrieve Values:** Get the normalized `f64` values for the top and bottom data at that index.
3.  **Color Calculation (New):**
    *   If `fade_effect` is true:
        *   Calculate `fade_factor = (x - inner_area.left()) as f64 / inner_area.width() as f64`.
        *   Interpolate the `top_style.fg` and `bottom_style.fg` towards black (or dim them) based on `fade_factor`.
        *   Newest data (rightmost) should be 100% opacity. Oldest data (leftmost) should be dim.

### Phase 3: Rendering (Branch by Mode)

#### Branch A: If `self.mode` is `HighResBraille`

1.  **Calculate Dot Capacity:** The total available height in dots for one side is `max_char_height * 4`.
2.  **Calculate Needed Dots:** For the current data point, calculate `(normalized_value * total_available_dots) as u16`.

**Rendering the Top Half (Upwards):**
Iterate from `y = center_y - 1` upwards to `inner_area.top()`. Maintain a counter of `dots_remaining_to_draw`.

  * If `dots_remaining_to_draw >= 4`: Draw the full left-column character `⡇` at `(x,y)` using `top_style`. Subtract 4 from remaining dots.
  * Else if `dots_remaining_to_draw > 0`: This is the peak. Call `get_thin_braille_fill(dots_remaining)` to get the partial character (e.g., `⡆`). Draw it at `(x,y)`. Set remaining dots to 0.
  * Else: Stop iterating upwards.

**Rendering the Bottom Half (Downwards):**
Repeat the exact logic above, but iterate from `y = center_y` downwards to `inner_area.bottom()`, using `bottom_style` and the bottom data value.

#### Branch B: If `self.mode` is `UltraThinBlock`

1.  **Calculate Needed Rows:** For the current data point, calculate how many full character rows are needed: `(normalized_value * max_char_height as f64) as u16`.

**Rendering the Top Half (Upwards):**
Iterate `i` from 0 up to `needed_rows`.

  * Calculate target `y = (center_y - 1) - i`.
  * Ensure `y` is not outside bounds (`y < inner_area.top()`).
  * Draw the half-block character `▌` at `(x,y)` using `top_style`.

**Rendering the Bottom Half (Downwards):**
Iterate `i` from 0 up to `needed_rows`.

  * Calculate target `y = center_y + i`.
  * Ensure `y` is not outside bounds (`y >= inner_area.bottom()`).
  * Draw the half-block character `▌` at `(x,y)` using `bottom_style`.

-----

## 6. Demo Application Specification

To demonstrate the widget's capabilities, a demo application will be built with the following features:

### 6.1 Data Sources
The application will monitor system resources using `sysinfo`.
- **CPU Usage**: Aggregated global CPU usage percentage.
- **Memory Usage**: RAM usage percentage.

### 6.2 Configuration
The user can dynamically configure the widget at runtime:
- **Source Selection**:
    - Top Half: Toggle between CPU and Memory.
    - Bottom Half: Toggle between CPU and Memory.
    - *Supported Combinations*: CPU/Mem, CPU/CPU, Mem/Mem, Mem/CPU.
- **Tick Rate**: Adjust the update frequency (speed of the chart).
    - Increase/Decrease speed (e.g., 50ms to 1000ms intervals).
- **Color Customization**:
    - Cycle through predefined color palettes for Top and Bottom graphs independently.
- **Visual Effects (New)**:
    - Toggle **Horizontal Fade** (Time-decay opacity).

### 6.3 Controls
- `q`: Quit
- `1`: Toggle Top Source (CPU <-> Mem)
- `2`: Toggle Bottom Source (CPU <-> Mem)
- `+`: Increase Tick Speed (Faster)
- `-`: Decrease Tick Speed (Slower)
- `c`: Cycle Colors
- `m`: Toggle Mode (Braille <-> Block)
- `f`: Toggle Fade Effect