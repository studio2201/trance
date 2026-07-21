<p align="center">
  <a href="https://crateria.github.io/">
    <img src="assets/crateria-header.jpg" alt="Trance Banner" width="100%">
  </a>
</p>

<h1 align="center">Trance</h1>

<p align="center">
  <b>Modular, High-Performance Wayland Screensaver & Idle Engine in Rust</b>
</p>

<p align="center">
  <a href="https://github.com/studio2201/trance/actions/workflows/ci.yml"><img src="https://github.com/studio2201/trance/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/studio2201/trance/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/studio2201/trance"><img src="https://img.shields.io/badge/language-Rust-orange.svg" alt="Rust"></a>
</p>

---

### Instant Installation (One Line)

```bash
curl -fsSL https://crateria.github.io/install.sh | sh
```

---

### Instant Quick Start (One Command)

Preview the default **Beams** screensaver in high-fps fullscreen mode:

```bash
trance preview beams
```

Toggle interactive controls or system diagnostics:

```bash
trance interactive   # Launch interactive TUI control panel
trance doctor        # Run self-healing system diagnostics
```

---

### Showcase

<p align="center">
  <img src="assets/beams.webp" alt="Beams Saver" width="48%">
  <img src="assets/ripple.webp" alt="Ripple Saver" width="48%">
</p>

---

### Core Architecture & Ecosystem Docs

For deeper technical specifications, protocol compliance, IPC wire formats, and plugin development guides:

- 📖 **[Architecture Overview](ARCHITECTURE.md)** — Wayland layer-shell, D-Bus interfaces, and out-of-process `memfd_create` IPC pipeline.
- ⚙️ **[Plugin Developer Guide](docs/PLUGINS.md)** — How to write custom high-performance terminal & GPU screensavers in Rust.
- 🛠️ **[CLI & Daemon Manual](docs/USAGE.md)** — Full command reference and `config.yaml` customization options.

---

### License

Distributed under the Apache 2.0 License. See [LICENSE](LICENSE) for details.
