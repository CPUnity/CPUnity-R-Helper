// The settings of the log viewer window (text sizes, what is drawn, how the mouse behaves, units,
// colours). Pure data: kept as one line of text in the config file, sanitised on the way in, so a
// hand-edited or older file can never produce a broken window. The window itself is `ui::log_window`.

/// Where the panel with the line switches and the numbers sits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
    Hidden,
}

/// Dots on the readings of a line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dots {
    /// Only when zoomed in far enough that they do not crowd each other.
    Auto,
    Always,
    Never,
}

/// What the vertical axis of a graph spans.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum YMode {
    /// What is on screen (the axis moves while the log is moved).
    Visible,
    /// The whole log (the axis stays put while the log is moved).
    Whole,
    /// Like `Visible`, but always reaching down to zero.
    Zero,
}

/// What the mouse wheel does over a graph.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wheel {
    Zoom,
    Move,
}

pub const PERCENT_MIN: f32 = 60.0;
pub const PERCENT_MAX: f32 = 220.0;

#[derive(Clone, PartialEq, Debug)]
pub struct Options {
    // ---- text sizes (percent of the standard size of this window) ----
    pub size_all: f32,
    pub size_side: f32,
    pub size_toolbar: f32,
    pub size_options: f32,
    pub size_titles: f32,
    pub size_axis: f32,
    pub size_readout: f32,
    pub size_overview: f32,
    // ---- layout ----
    pub side: Side,
    pub side_hint: bool,
    pub overview: bool,
    pub overview_height: f32,
    pub chart_gap: f32,
    pub chart_corner: f32,
    pub chart_bg: f32,
    pub graph_on: [bool; 4],
    // ---- lines ----
    pub line_width: f32,
    pub glow: bool,
    pub fill: bool,
    pub fill_percent: f32,
    pub dots: Dots,
    pub dot_size: f32,
    pub step: bool,
    pub hover_highlight: bool,
    pub avg_line: bool,
    pub peaks: bool,
    // ---- axes ----
    pub y_mode: YMode,
    pub grid_h: bool,
    pub grid_v: bool,
    pub grid_percent: f32,
    pub seconds: bool,
    // ---- crosshair ----
    pub crosshair: bool,
    pub readout: bool,
    pub guide_h: bool,
    pub snap: bool,
    pub readout_opacity: f32,
    // ---- mouse ----
    pub wheel: Wheel,
    pub zoom_speed: f32,
    pub wheel_invert: bool,
    // ---- units and numbers ----
    pub fahrenheit: bool,
    pub stats_whole: bool,
    pub col_low: bool,
    pub col_avg: bool,
    pub col_high: bool,
    pub col_last: bool,
    pub temp_decimals: u8,
    pub power_decimals: u8,
    pub row_spacing: f32,
    // ---- window ----
    pub on_top: bool,
    /// Line colours the user picked: (column name of the line, red, green, blue).
    pub colours: Vec<(String, [u8; 3])>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            size_all: 100.0,
            size_side: 100.0,
            size_toolbar: 100.0,
            size_options: 100.0,
            size_titles: 100.0,
            size_axis: 100.0,
            size_readout: 100.0,
            size_overview: 100.0,
            side: Side::Left,
            side_hint: true,
            overview: true,
            overview_height: 78.0,
            chart_gap: 8.0,
            chart_corner: 8.0,
            chart_bg: 16.0,
            graph_on: [true; 4],
            line_width: 1.8,
            glow: true,
            fill: true,
            fill_percent: 24.0,
            dots: Dots::Auto,
            dot_size: 2.5,
            step: false,
            hover_highlight: true,
            avg_line: false,
            peaks: false,
            y_mode: YMode::Visible,
            grid_h: true,
            grid_v: true,
            grid_percent: 100.0,
            seconds: false,
            crosshair: true,
            readout: true,
            guide_h: false,
            snap: false,
            readout_opacity: 92.0,
            wheel: Wheel::Zoom,
            zoom_speed: 100.0,
            wheel_invert: false,
            fahrenheit: false,
            stats_whole: false,
            col_low: true,
            col_avg: true,
            col_high: true,
            col_last: false,
            temp_decimals: 0,
            power_decimals: 1,
            row_spacing: 7.0,
            on_top: false,
            colours: Vec::new(),
        }
    }
}

fn clamp(value: f32, lo: f32, hi: f32) -> f32 {
    if value.is_finite() {
        value.clamp(lo, hi)
    } else {
        lo
    }
}

impl Options {
    /// Every value pulled back into what the window can draw (a wrong number in the file is
    /// harmless).
    pub fn sanitized(mut self) -> Options {
        for size in [
            &mut self.size_all,
            &mut self.size_side,
            &mut self.size_toolbar,
            &mut self.size_options,
            &mut self.size_titles,
            &mut self.size_axis,
            &mut self.size_readout,
            &mut self.size_overview,
        ] {
            *size = clamp(*size, PERCENT_MIN, PERCENT_MAX);
        }
        self.overview_height = clamp(self.overview_height, 40.0, 200.0);
        self.chart_gap = clamp(self.chart_gap, 0.0, 30.0);
        self.chart_corner = clamp(self.chart_corner, 0.0, 24.0);
        self.chart_bg = clamp(self.chart_bg, 0.0, 60.0);
        self.line_width = clamp(self.line_width, 0.5, 5.0);
        self.fill_percent = clamp(self.fill_percent, 0.0, 100.0);
        self.dot_size = clamp(self.dot_size, 1.0, 8.0);
        self.grid_percent = clamp(self.grid_percent, 0.0, 300.0);
        self.readout_opacity = clamp(self.readout_opacity, 30.0, 100.0);
        self.zoom_speed = clamp(self.zoom_speed, 20.0, 400.0);
        self.row_spacing = clamp(self.row_spacing, 0.0, 24.0);
        self.temp_decimals = self.temp_decimals.min(2);
        self.power_decimals = self.power_decimals.min(2);
        // at least one column of numbers, and one graph
        if !(self.col_low || self.col_avg || self.col_high || self.col_last) {
            self.col_avg = true;
        }
        if !self.graph_on.iter().any(|on| *on) {
            self.graph_on = [true; 4];
        }
        self.colours.retain(|(name, _)| !name.is_empty());
        self
    }

    /// Size of one kind of text as a factor (1.0 = the standard size of the window), the master
    /// size included.
    pub fn factor(&self, percent: f32) -> f32 {
        percent / 100.0 * self.size_all / 100.0
    }

    /// The colour picked for a line, if any.
    pub fn colour_of(&self, column: &str) -> Option<[u8; 3]> {
        self.colours.iter().find(|(name, _)| name == column).map(|(_, rgb)| *rgb)
    }

    pub fn set_colour(&mut self, column: &str, rgb: [u8; 3]) {
        match self.colours.iter_mut().find(|(name, _)| name == column) {
            Some(entry) => entry.1 = rgb,
            None => self.colours.push((column.to_string(), rgb)),
        }
    }

    pub fn reset_colour(&mut self, column: &str) {
        self.colours.retain(|(name, _)| name != column);
    }

    /// Only the sizes and the layout go back (colours, units and the like are kept).
    pub fn preset(&mut self, which: Preset) {
        let keep = self.clone();
        let (all, graphs) = match which {
            Preset::Compact => (85.0, 90.0),
            Preset::Standard => (100.0, 100.0),
            Preset::Large => (125.0, 125.0),
            Preset::Huge => (160.0, 160.0),
        };
        *self = Options {
            size_all: all,
            size_titles: graphs,
            size_axis: graphs,
            size_readout: graphs,
            ..keep
        };
    }

    // ---- as one line of text ----

    pub fn encode(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut num = |key: &str, value: f32| parts.push(format!("{key}={value}"));
        num("size_all", self.size_all);
        num("size_side", self.size_side);
        num("size_toolbar", self.size_toolbar);
        num("size_options", self.size_options);
        num("size_titles", self.size_titles);
        num("size_axis", self.size_axis);
        num("size_readout", self.size_readout);
        num("size_overview", self.size_overview);
        num("overview_height", self.overview_height);
        num("chart_gap", self.chart_gap);
        num("chart_corner", self.chart_corner);
        num("chart_bg", self.chart_bg);
        num("line_width", self.line_width);
        num("fill_percent", self.fill_percent);
        num("dot_size", self.dot_size);
        num("grid_percent", self.grid_percent);
        num("readout_opacity", self.readout_opacity);
        num("zoom_speed", self.zoom_speed);
        num("row_spacing", self.row_spacing);
        let mut flag = |key: &str, value: bool| parts.push(format!("{key}={}", u8::from(value)));
        flag("side_hint", self.side_hint);
        flag("overview", self.overview);
        flag("glow", self.glow);
        flag("fill", self.fill);
        flag("step", self.step);
        flag("hover_highlight", self.hover_highlight);
        flag("avg_line", self.avg_line);
        flag("peaks", self.peaks);
        flag("grid_h", self.grid_h);
        flag("grid_v", self.grid_v);
        flag("seconds", self.seconds);
        flag("crosshair", self.crosshair);
        flag("readout", self.readout);
        flag("guide_h", self.guide_h);
        flag("snap", self.snap);
        flag("wheel_invert", self.wheel_invert);
        flag("fahrenheit", self.fahrenheit);
        flag("stats_whole", self.stats_whole);
        flag("col_low", self.col_low);
        flag("col_avg", self.col_avg);
        flag("col_high", self.col_high);
        flag("col_last", self.col_last);
        flag("on_top", self.on_top);
        parts.push(format!("temp_decimals={}", self.temp_decimals));
        parts.push(format!("power_decimals={}", self.power_decimals));
        parts.push(format!(
            "side={}",
            match self.side {
                Side::Left => "left",
                Side::Right => "right",
                Side::Hidden => "hidden",
            }
        ));
        parts.push(format!(
            "dots={}",
            match self.dots {
                Dots::Auto => "auto",
                Dots::Always => "always",
                Dots::Never => "never",
            }
        ));
        parts.push(format!(
            "y_mode={}",
            match self.y_mode {
                YMode::Visible => "visible",
                YMode::Whole => "whole",
                YMode::Zero => "zero",
            }
        ));
        parts.push(format!(
            "wheel={}",
            match self.wheel {
                Wheel::Zoom => "zoom",
                Wheel::Move => "move",
            }
        ));
        let graphs: String = self.graph_on.iter().map(|on| if *on { '1' } else { '0' }).collect();
        parts.push(format!("graph_on={graphs}"));
        for (name, [r, g, b]) in &self.colours {
            parts.push(format!("c.{name}={r:02x}{g:02x}{b:02x}"));
        }
        parts.join(";")
    }

    /// Reads what `encode` wrote. Anything unknown or unreadable is ignored (the default stays).
    pub fn decode(text: &str) -> Options {
        let mut o = Options::default();
        for part in text.split(';') {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            let number = value.parse::<f32>().ok().filter(|v| v.is_finite());
            let flag = match value {
                "1" | "true" => Some(true),
                "0" | "false" => Some(false),
                _ => None,
            };
            if let Some(name) = key.strip_prefix("c.") {
                if let Some(rgb) = parse_hex(value) {
                    o.set_colour(name, rgb);
                }
                continue;
            }
            match key {
                "size_all" => set(&mut o.size_all, number),
                "size_side" => set(&mut o.size_side, number),
                "size_toolbar" => set(&mut o.size_toolbar, number),
                "size_options" => set(&mut o.size_options, number),
                "size_titles" => set(&mut o.size_titles, number),
                "size_axis" => set(&mut o.size_axis, number),
                "size_readout" => set(&mut o.size_readout, number),
                "size_overview" => set(&mut o.size_overview, number),
                "overview_height" => set(&mut o.overview_height, number),
                "chart_gap" => set(&mut o.chart_gap, number),
                "chart_corner" => set(&mut o.chart_corner, number),
                "chart_bg" => set(&mut o.chart_bg, number),
                "line_width" => set(&mut o.line_width, number),
                "fill_percent" => set(&mut o.fill_percent, number),
                "dot_size" => set(&mut o.dot_size, number),
                "grid_percent" => set(&mut o.grid_percent, number),
                "readout_opacity" => set(&mut o.readout_opacity, number),
                "zoom_speed" => set(&mut o.zoom_speed, number),
                "row_spacing" => set(&mut o.row_spacing, number),
                "side_hint" => setb(&mut o.side_hint, flag),
                "overview" => setb(&mut o.overview, flag),
                "glow" => setb(&mut o.glow, flag),
                "fill" => setb(&mut o.fill, flag),
                "step" => setb(&mut o.step, flag),
                "hover_highlight" => setb(&mut o.hover_highlight, flag),
                "avg_line" => setb(&mut o.avg_line, flag),
                "peaks" => setb(&mut o.peaks, flag),
                "grid_h" => setb(&mut o.grid_h, flag),
                "grid_v" => setb(&mut o.grid_v, flag),
                "seconds" => setb(&mut o.seconds, flag),
                "crosshair" => setb(&mut o.crosshair, flag),
                "readout" => setb(&mut o.readout, flag),
                "guide_h" => setb(&mut o.guide_h, flag),
                "snap" => setb(&mut o.snap, flag),
                "wheel_invert" => setb(&mut o.wheel_invert, flag),
                "fahrenheit" => setb(&mut o.fahrenheit, flag),
                "stats_whole" => setb(&mut o.stats_whole, flag),
                "col_low" => setb(&mut o.col_low, flag),
                "col_avg" => setb(&mut o.col_avg, flag),
                "col_high" => setb(&mut o.col_high, flag),
                "col_last" => setb(&mut o.col_last, flag),
                "on_top" => setb(&mut o.on_top, flag),
                "temp_decimals" => {
                    if let Ok(v) = value.parse::<u8>() {
                        o.temp_decimals = v;
                    }
                }
                "power_decimals" => {
                    if let Ok(v) = value.parse::<u8>() {
                        o.power_decimals = v;
                    }
                }
                "side" => match value {
                    "left" => o.side = Side::Left,
                    "right" => o.side = Side::Right,
                    "hidden" => o.side = Side::Hidden,
                    _ => {}
                },
                "dots" => match value {
                    "auto" => o.dots = Dots::Auto,
                    "always" => o.dots = Dots::Always,
                    "never" => o.dots = Dots::Never,
                    _ => {}
                },
                "y_mode" => match value {
                    "visible" => o.y_mode = YMode::Visible,
                    "whole" => o.y_mode = YMode::Whole,
                    "zero" => o.y_mode = YMode::Zero,
                    _ => {}
                },
                "wheel" => match value {
                    "zoom" => o.wheel = Wheel::Zoom,
                    "move" => o.wheel = Wheel::Move,
                    _ => {}
                },
                "graph_on" => {
                    let chars: Vec<char> = value.chars().collect();
                    if chars.len() == 4 && chars.iter().all(|c| *c == '0' || *c == '1') {
                        for (slot, c) in o.graph_on.iter_mut().zip(chars) {
                            *slot = c == '1';
                        }
                    }
                }
                _ => {}
            }
        }
        o.sanitized()
    }
}

fn set(slot: &mut f32, value: Option<f32>) {
    if let Some(v) = value {
        *slot = v;
    }
}

fn setb(slot: &mut bool, value: Option<bool>) {
    if let Some(v) = value {
        *slot = v;
    }
}

/// `ff8040` to its three channels.
pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let text = text.trim().trim_start_matches('#');
    if text.len() != 6 || !text.is_ascii() {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&text[i..i + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    Compact,
    Standard,
    Large,
    Huge,
}

/// Degrees Celsius as Fahrenheit.
pub fn fahrenheit(celsius: f32) -> f32 {
    celsius * 9.0 / 5.0 + 32.0
}

/// A line drawn as steps: each reading holds until the next one (how a sensor really behaves),
/// instead of a slope between them.
pub fn stepped(points: &[(f64, f32)]) -> Vec<(f64, f32)> {
    let mut out = Vec::with_capacity(points.len() * 2);
    for (i, p) in points.iter().enumerate() {
        if i > 0 {
            out.push((p.0, points[i - 1].1));
        }
        out.push(*p);
    }
    out
}

/// The number as text with a fixed count of decimals.
pub fn number_text(value: f32, decimals: u8) -> String {
    format!("{:.*}", usize::from(decimals.min(3)), value)
}

/// The earliest highest and lowest reading of a set: (time, value) of each.
pub fn extremes(points: &[(f64, f32)]) -> Option<((f64, f32), (f64, f32))> {
    let mut iter = points.iter().filter(|p| p.1.is_finite());
    let first = *iter.next()?;
    let (mut lo, mut hi) = (first, first);
    for p in iter {
        if p.1 < lo.1 {
            lo = *p;
        }
        if p.1 > hi.1 {
            hi = *p;
        }
    }
    Some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_survives_a_round_trip() {
        let o = Options::default();
        assert_eq!(Options::decode(&o.encode()), o);
    }

    #[test]
    fn changed_values_survive_a_round_trip() {
        let mut o = Options::default();
        o.size_side = 140.0;
        o.size_axis = 80.0;
        o.side = Side::Right;
        o.dots = Dots::Never;
        o.y_mode = YMode::Whole;
        o.wheel = Wheel::Move;
        o.step = true;
        o.fahrenheit = true;
        o.col_last = true;
        o.col_low = false;
        o.temp_decimals = 2;
        o.graph_on = [true, false, true, false];
        o.set_colour("cpu_temp", [255, 128, 0]);
        assert_eq!(Options::decode(&o.encode()), o);
    }

    #[test]
    fn garbage_is_ignored() {
        let o =
            Options::decode("size_all=banana;;=;side=up;zzz=1;graph_on=12;line_width=NaN;c.x=zz");
        assert_eq!(o, Options::default());
        assert_eq!(Options::decode(""), Options::default());
    }

    #[test]
    fn numbers_are_pulled_into_range() {
        let o = Options::decode(
            "size_all=9000;size_side=-5;line_width=100;zoom_speed=0;temp_decimals=9",
        );
        assert_eq!(o.size_all, PERCENT_MAX);
        assert_eq!(o.size_side, PERCENT_MIN);
        assert_eq!(o.line_width, 5.0);
        assert_eq!(o.zoom_speed, 20.0);
        assert_eq!(o.temp_decimals, 2);
    }

    #[test]
    fn never_without_a_column_or_a_graph() {
        let o = Options::decode("col_low=0;col_avg=0;col_high=0;col_last=0;graph_on=0000");
        assert!(o.col_avg);
        assert_eq!(o.graph_on, [true; 4]);
    }

    #[test]
    fn factor_multiplies_the_master_size() {
        let mut o = Options::default();
        assert!((o.factor(o.size_side) - 1.0).abs() < 1e-6);
        o.size_all = 150.0;
        o.size_side = 80.0;
        assert!((o.factor(o.size_side) - 1.2).abs() < 1e-6);
    }

    #[test]
    fn colours_are_set_replaced_and_reset() {
        let mut o = Options::default();
        assert_eq!(o.colour_of("gpu_temp"), None);
        o.set_colour("gpu_temp", [1, 2, 3]);
        o.set_colour("gpu_temp", [4, 5, 6]);
        assert_eq!(o.colour_of("gpu_temp"), Some([4, 5, 6]));
        assert_eq!(o.colours.len(), 1);
        o.reset_colour("gpu_temp");
        assert_eq!(o.colour_of("gpu_temp"), None);
    }

    #[test]
    fn hex_colours() {
        assert_eq!(parse_hex("ff8040"), Some([255, 128, 64]));
        assert_eq!(parse_hex("#00ff00"), Some([0, 255, 0]));
        assert_eq!(parse_hex("ff80"), None);
        assert_eq!(parse_hex("gggggg"), None);
        assert_eq!(parse_hex("é12345"), None);
    }

    #[test]
    fn presets_change_sizes_only() {
        let mut o = Options::default();
        o.fahrenheit = true;
        o.step = true;
        o.size_side = 170.0;
        o.preset(Preset::Large);
        assert_eq!(o.size_all, 125.0);
        assert_eq!(o.size_titles, 125.0);
        assert!(o.fahrenheit && o.step);
        assert_eq!(o.size_side, 170.0);
        o.preset(Preset::Standard);
        assert_eq!(o.size_all, 100.0);
    }

    #[test]
    fn fahrenheit_values() {
        assert_eq!(fahrenheit(0.0), 32.0);
        assert_eq!(fahrenheit(100.0), 212.0);
        assert!((fahrenheit(-40.0) + 40.0).abs() < 1e-4);
    }

    #[test]
    fn steps_hold_each_reading_until_the_next() {
        assert!(stepped(&[]).is_empty());
        assert_eq!(stepped(&[(1.0, 5.0)]), vec![(1.0, 5.0)]);
        assert_eq!(
            stepped(&[(0.0, 1.0), (2.0, 3.0), (5.0, 2.0)]),
            vec![(0.0, 1.0), (2.0, 1.0), (2.0, 3.0), (5.0, 3.0), (5.0, 2.0)]
        );
    }

    #[test]
    fn decimals() {
        assert_eq!(number_text(45.678, 0), "46");
        assert_eq!(number_text(45.678, 1), "45.7");
        assert_eq!(number_text(45.678, 2), "45.68");
    }

    #[test]
    fn extremes_find_the_first_lowest_and_highest() {
        assert_eq!(extremes(&[]), None);
        let points = [(0.0, 5.0), (1.0, 2.0), (2.0, 9.0), (3.0, 2.0), (4.0, 9.0)];
        assert_eq!(extremes(&points), Some(((1.0, 2.0), (2.0, 9.0))));
        assert_eq!(extremes(&[(0.0, f32::NAN), (1.0, 3.0)]), Some(((1.0, 3.0), (1.0, 3.0))));
    }
}
