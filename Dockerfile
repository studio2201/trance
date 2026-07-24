# Optional Alpine multi-stage build for tooling and CI smoke images.
# Desktop users should install native packages from idlescreen.github.io/packages.

FROM alpine:3.20 AS builder

RUN apk add --no-cache \
    rust cargo build-base pkgconfig dbus-dev wayland-dev libxkbcommon-dev linux-headers

WORKDIR /app
COPY . .

RUN cargo build --release -p trance-daemon -p trance-cli

FROM alpine:3.20

RUN apk add --no-cache dbus wayland-libs libxkbcommon ca-certificates

COPY --from=builder /app/target/release/trance-daemon /usr/bin/trance-daemon
COPY --from=builder /app/target/release/trance /usr/bin/trance

ENV WAYLAND_DISPLAY=wayland-0
ENV XDG_CONFIG_HOME=/root/.config

# Daemon requires a real Wayland session; this image is for packaging smoke
# and tooling, not unattended fullscreen display without a compositor.
ENTRYPOINT ["/usr/bin/trance-daemon"]
