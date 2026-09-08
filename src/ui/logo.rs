/// Plain-text logo shown in the picker's directory-input screen.
///
/// The artwork lives in `ascii-art.txt`, embedded at compile time. It is plain
/// text, so it renders with the normal text style and needs no color parsing.
const LOGO: &str = include_str!("ascii-art.txt");

/// A copy of the artwork ready to draw.
pub struct Logo {
    /// One line per text row of the artwork, without trailing whitespace.
    pub lines: Vec<&'static str>,
    /// The widest line, in characters, used to centre the artwork.
    pub width: usize,
}

/// Load the logo and measure it.
pub fn logo() -> Logo {
    let lines: Vec<&'static str> = LOGO.lines().map(str::trim_end).collect();
    let width = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    Logo { lines, width }
}
