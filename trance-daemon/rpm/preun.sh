#!/bin/sh
# RPM %preun — $1 is count remaining after this transaction
# (0 = full uninstall, 1+ = upgrade).
#
# Do NOT stop the user service on upgrade (see post.sh / posttrans.sh).
set -u

# shellcheck disable=SC1091
if [ -f /usr/lib/trance/user-service-lib.sh ]; then
    . /usr/lib/trance/user-service-lib.sh
else
    is_desktop_uid() {
        case "$1" in ''|*[!0-9]*) return 1 ;; esac
        [ "$1" -ge 1000 ]
    }
    for_each_user_session() {
        _cb="$1"
        command -v loginctl >/dev/null 2>&1 || return 0
        command -v systemctl >/dev/null 2>&1 || return 0
        loginctl list-users --no-legend 2>/dev/null | while read -r uid user _rest; do
            is_desktop_uid "$uid" || continue
            [ -n "$user" ] || continue
            [ -d "/run/user/$uid" ] || continue
            [ -S "/run/user/$uid/bus" ] || continue
            "$_cb" "$uid" "$user" || true
        done
    }
    _user_systemctl() {
        _uid="$1"; _user="$2"; shift 2
        if command -v runuser >/dev/null 2>&1; then
            runuser -u "$_user" -- env \
                XDG_RUNTIME_DIR="/run/user/$_uid" \
                DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$_uid/bus" \
                systemctl --user "$@" 2>/dev/null && return 0
        fi
        systemctl --user --machine="${_user}@" "$@" 2>/dev/null || true
    }
    try_stop_trance() {
        _user_systemctl "$1" "$2" stop trance-daemon.service || true
    }
fi

# Upgrade: leave the daemon alone.
if [ "${1:-0}" -ne 0 ]; then
    exit 0
fi

# Full uninstall only.
try_disable() {
    _user_systemctl "$1" "$2" disable trance-daemon.service || true
}

for_each_user_session try_stop_trance
for_each_user_session try_disable

exit 0
