#!/bin/sh
# Give the screenshot shortcuts back to GNOME for every logged-in user (best effort).
# Users who weren't logged in can do it by hand: see "Give the shortcuts back" in the README.
set -e
case "$1" in
  remove|purge)
    for uid in $(loginctl list-users --no-legend 2>/dev/null | awk '{print $1}'); do
      user=$(id -nu "$uid" 2>/dev/null) || continue
      home=$(getent passwd "$user" | cut -d: -f6)
      runuser -u "$user" -- env HOME="$home" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" \
        /usr/bin/rshot restore-shortcuts || true
    done
    ;;
esac
exit 0
