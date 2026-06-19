use ratatui::style::{Color, Modifier, Style};

pub const HEADERS_POOL: &[&str] = &[
    "press (p) to preview screensaver",
    "press (t) to cycle accents",
    "press (d)aemon to toggle background daemon",
    "press (i)dle to toggle idle activation",
    "press (r) to enter trance mode",
    "press (q) to exit config tool",
    "press (s)elect to pin a screensaver",
    "adjust idle time with (m)ore/(l)ess time",
    "is it art or is it just a screensaver?",
    "please do not touch the glass",
    "preventing burn-in since 1989",
    "flying toasters not included",
    "in a trance of code",
    "watching accretion disks orbit...",
    "compiling virtual starfields...",
    "simulating predatory boid swarms...",
    "watching the doom fire rise...",
    "matrix rain glyphs are falling...",
    "a screensaver is a movie for an idle pc",
    "the monitor is resting its phosphors",
];

pub fn get_quote(idx: usize) -> &'static str {
    HEADERS_POOL[idx % HEADERS_POOL.len()]
}

pub fn parse_quote_spans(
    quote: &str,
    accent: Color,
    fg: Color,
) -> Vec<ratatui::text::Span<'static>> {
    let mut spans = Vec::new();
    let mut current = quote;
    while let Some(start_idx) = current.find('(') {
        if let Some(end_idx) = current[start_idx..].find(')') {
            let absolute_end = start_idx + end_idx;
            let before_part = &current[..start_idx];
            let key_part = &current[start_idx..=absolute_end];

            if !before_part.is_empty() {
                spans.push(ratatui::text::Span::styled(
                    before_part.to_string(),
                    Style::default().fg(fg),
                ));
            }
            spans.push(ratatui::text::Span::styled(
                key_part.to_string(),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ));

            current = &current[absolute_end + 1..];
        } else {
            break;
        }
    }
    if !current.is_empty() {
        spans.push(ratatui::text::Span::styled(
            current.to_string(),
            Style::default().fg(fg),
        ));
    }
    spans
}

pub fn get_screensaver_description(name: &str) -> &'static str {
    match name {
        "beams" => "sweeps 4 colored spotlight cones over a rising dust starfield. spotlight speeds and density scale dynamically based on current memory load and active processor usage.",
        "bursts" => "vector-drawn city skyline backdrop with colorful rocket firework particles launching upwards and bursting into gravity-driven paths.",
        "chaos" => "spring-back glitch rendering of the local76 logo utilizing randomized chromatic aberration rgb color split-displacement effects.",
        "cosmos" => "simulates an accretion universe lifecycle, drawing gas disks, planet orbit mechanics, star supernovas, and black hole singular collapses.",
        "glyphs" => "classic matrix rain animation down terminal columns, interspersed with details of current interfaces, kernel version, hostname, and active processes.",
        "gnats" => "triadic color fly/insect swarm simulating predator-prey tracking vectors. spawns predator boids relative to memory pressure.",
        "storm" => "cold rain storm simulation with shifting wind directions, random brightness lightning flashes, and animated forest animals walking between trees.",
        _ => "no description available for this screensaver wrapper."
    }
}

pub fn parse_shortcut_spans(
    shortcuts: &[&str],
    accent: Color,
    fg: Color,
    dim: Color,
) -> Vec<ratatui::text::Span<'static>> {
    let mut footer_spans = vec![ratatui::text::Span::raw(" ")];
    for (i, txt) in shortcuts.iter().enumerate() {
        if i > 0 {
            footer_spans.push(ratatui::text::Span::styled(" | ", Style::default().fg(dim)));
        }
        if let Some(start_idx) = txt.find('(') {
            if let Some(end_idx) = txt[start_idx..].find(')') {
                let absolute_end = start_idx + end_idx;
                let before = &txt[..start_idx];
                let key_part = &txt[start_idx..=absolute_end];
                let after = &txt[absolute_end + 1..];
                if !before.is_empty() {
                    footer_spans.push(ratatui::text::Span::styled(
                        before.to_string(),
                        Style::default().fg(fg),
                    ));
                }
                footer_spans.push(ratatui::text::Span::styled(
                    key_part.to_string(),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ));
                if !after.is_empty() {
                    footer_spans.push(ratatui::text::Span::styled(
                        after.to_string(),
                        Style::default().fg(fg),
                    ));
                }
            } else {
                footer_spans.push(ratatui::text::Span::styled(
                    (*txt).to_string(),
                    Style::default().fg(fg),
                ));
            }
        } else {
            footer_spans.push(ratatui::text::Span::styled(
                (*txt).to_string(),
                Style::default().fg(fg),
            ));
        }
    }
    footer_spans
}
