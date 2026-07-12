<p align="center">
  <a href="https://crateria.github.io/">
    <img src="assets/crateria-header.jpg" alt="Crateria" width="100%">
  </a>
</p>

# Trance

[![CI](https://github.com/crateria/trance/actions/workflows/ci.yml/badge.svg)](https://github.com/crateria/trance/actions/workflows/ci.yml)

Wayland-native modular screensaver daemon and desktop suite for Linux.

## Documentation & Installation

For full installation guides, D-Bus API documentation, configurator tools, and TUI controls, visit:
👉 **[https://crateria.github.io/](https://crateria.github.io/)**

## Quick Start (Build & Run)

```bash
# Clone the repository
git clone https://github.com/crateria/trance.git
cd trance

# Build workspace release binaries
cargo build --release

# Run the user-session daemon
./target/release/trance-daemon

# Control screensaver state via CLI
./target/release/trance status
```

## License

[Apache-2.0](LICENSE) · Copyright 2026 Crateria
