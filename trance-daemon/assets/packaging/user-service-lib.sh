#!/bin/sh
# Shared helpers for deb/rpm maintainer scripts.
# All operations are best-effort: never abort package install/upgrade.
# shellcheck disable=SC2039,SC3043

# Human (desktop) UIDs only. Skip root and system accounts — they often show
# up in loginctl during `sudo dnf` but never run the screensaver unit.
is_desktop_uid() {
    case "$1" in
        ''|*[!0-9]*) return 1 ;;
    esac
    # Typical SYS_UID_MAX is 999 on Fedora/Debian.
    [ "$1" -ge 1000 ]
}

# Iterate logged-in desktop users that have a usable user bus.
# Calls: for_each_user_session <callback>
# callback receives: uid user
for_each_user_session() {
    _cb="$1"
    command -v loginctl >/dev/null 2>&1 || return 0
    command -v systemctl >/dev/null 2>&1 || return 0

    # Columns vary by systemd version; take first two tokens (uid, user).
    loginctl list-users --no-legend 2>/dev/null | while read -r uid user _rest; do
        is_desktop_uid "$uid" || continue
        [ -n "$user" ] || continue
        [ -d "/run/user/$uid" ] || continue
        [ -S "/run/user/$uid/bus" ] || continue
        "$_cb" "$uid" "$user" || true
    done
}

_user_systemctl() {
    _uid="$1"
    _user="$2"
    shift 2
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "$_user" -- env \
            XDG_RUNTIME_DIR="/run/user/$_uid" \
            DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
            systemctl --user "$@" 2>/dev/null && return 0
    fi
    systemctl --user --machine="${_user}@" "$@" 2>/dev/null || true
}

_user_is_enabled() {
    _uid="$1"
    _user="$2"
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "$_user" -- env \
            XDG_RUNTIME_DIR="/run/user/$_uid" \
            DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
            systemctl --user is-enabled trance-daemon.service >/dev/null 2>&1
        return $?
    fi
    systemctl --user --machine="${_user}@" is-enabled trance-daemon.service >/dev/null 2>&1
}

_user_is_active() {
    _uid="$1"
    _user="$2"
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "$_user" -- env \
            XDG_RUNTIME_DIR="/run/user/$_uid" \
            DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
            systemctl --user is-active trance-daemon.service >/dev/null 2>&1
        return $?
    fi
    systemctl --user --machine="${_user}@" is-active trance-daemon.service >/dev/null 2>&1
}

try_reload_user_units() {
    _uid="$1"
    _user="$2"
    # Quiet: only reload when the unit is enabled or running for this user.
    if _user_is_enabled "$_uid" "$_user" || _user_is_active "$_uid" "$_user"; then
        _user_systemctl "$_uid" "$_user" daemon-reload || true
    fi
}

try_stop_trance() {
    _uid="$1"
    _user="$2"
    _user_systemctl "$_uid" "$_user" stop trance-daemon.service || true
}

# Apply upgrade: load new binary without the user running systemctl.
# Enabled → restart (or start if dead). Active-but-not-enabled → try-restart.
try_restart_trance() {
    _uid="$1"
    _user="$2"
    _user_systemctl "$_uid" "$_user" reset-failed trance-daemon.service || true
    if _user_is_enabled "$_uid" "$_user"; then
        echo "trance: applying upgrade for ${_user} (user service)"
        _user_systemctl "$_uid" "$_user" restart trance-daemon.service || true
        return 0
    fi
    if _user_is_active "$_uid" "$_user"; then
        echo "trance: applying upgrade for ${_user} (running unit)"
        _user_systemctl "$_uid" "$_user" try-restart trance-daemon.service || true
    fi
}

print_user_hint() {
    echo ""
    echo "  Note: trance-daemon is a *user* systemd service."
    echo "  If the screensaver is not running after install, as your desktop user:"
    echo "    systemctl --user enable --now trance-daemon"
    echo "  or:  trance doctor --fix"
    echo ""
}
