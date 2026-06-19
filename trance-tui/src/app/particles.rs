use crate::app::AppState;

#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub char: char,
    pub color_offset: usize,
}

pub fn toggle_trance_mode(state: &mut AppState, width: f64, height: f64) {
    if state.particles.is_empty() {
        struct SimpleRng {
            seed_state: u64,
        }
        impl SimpleRng {
            fn new() -> Self {
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                Self { seed_state: seed }
            }
            fn next_u64(&mut self) -> u64 {
                self.seed_state = self
                    .seed_state
                    .wrapping_mul(1664525)
                    .wrapping_add(1013904223);
                self.seed_state
            }
            fn next_f64(&mut self) -> f64 {
                (self.next_u64() & 0xFFFFFFFFFFFF) as f64 / 281474976710656.0
            }
        }

        let mut rng = SimpleRng::new();
        let chars = ['t', 'r', 'a', 'n', 'c', 'e'];
        let mut particles = Vec::new();
        for _ in 0..80 {
            let x = rng.next_f64() * width;
            let y = rng.next_f64() * height;
            let vx = (rng.next_f64() * 2.0 - 1.0) * 0.4;
            let vy = (rng.next_f64() * 2.0 - 1.0) * 0.2;
            let char_idx = (rng.next_u64() % 6) as usize;
            let color_offset = rng.next_u64() as usize;
            particles.push(Particle {
                x,
                y,
                vx,
                vy,
                char: chars[char_idx],
                color_offset,
            });
        }
        state.particles = particles;
        state.status_message = "trance mode activated.".to_string();
    } else {
        state.particles.clear();
        state.status_message = "trance mode deactivated.".to_string();
    }
    state.status_ttl_sec = 5;
}

pub fn update_particles(state: &mut AppState, width: f64, height: f64) {
    if !state.particles.is_empty() {
        for p in &mut state.particles {
            p.x += p.vx;
            p.y += p.vy;
            // Bounce off boundaries
            if p.x < 0.0 {
                p.x = 0.0;
                p.vx = -p.vx;
            } else if p.x >= width {
                p.x = width - 1.0;
                p.vx = -p.vx;
            }
            if p.y < 0.0 {
                p.y = 0.0;
                p.vy = -p.vy;
            } else if p.y >= height {
                p.y = height - 1.0;
                p.vy = -p.vy;
            }
        }
    }
}
