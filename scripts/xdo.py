#!/usr/bin/env python3
"""Minimal xdotool stand-in (xdotool isn't installed and there's no sudo).

  xdo.py key ctrl+super+F12     # synthesize a key combo via XTest (goes through the X server,
                                # so GNOME's keybinding grabs see it exactly like a real press)
  xdo.py key Return 0.12        # same, holding the keys down for 0.12 s before releasing
  xdo.py move 960 764           # warp the pointer to root coordinates (x, y)
  xdo.py active                 # name of the focused (_NET_ACTIVE_WINDOW) window
  xdo.py visible 'rshot overlay' # number of viewable windows whose name contains the text
"""
import ctypes, re, subprocess, sys, time

ALIASES = {"ctrl": "Control_L", "control": "Control_L", "shift": "Shift_L", "alt": "Alt_L",
           "super": "Super_L", "escape": "Escape", "esc": "Escape", "enter": "Return",
           "return": "Return", "space": "space", "print": "Print"}


def open_display():
    x11 = ctypes.cdll.LoadLibrary("libX11.so.6")
    x11.XOpenDisplay.restype = ctypes.c_void_p
    x11.XFlush.argtypes = [ctypes.c_void_p]
    x11.XCloseDisplay.argtypes = [ctypes.c_void_p]
    d = x11.XOpenDisplay(None)
    if not d:
        sys.exit("cannot open display")
    return x11, d


def key(combo: str, hold: float = 0) -> None:
    x11, d = open_display()
    xtst = ctypes.cdll.LoadLibrary("libXtst.so.6")
    x11.XStringToKeysym.restype = ctypes.c_ulong
    x11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    xtst.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
    codes = []
    for part in combo.split("+"):
        name = ALIASES.get(part.lower(), part)
        sym = x11.XStringToKeysym(name.encode())
        code = x11.XKeysymToKeycode(d, sym)
        if not code:
            sys.exit(f"unknown key: {part}")
        codes.append(code)
    for c in codes:
        xtst.XTestFakeKeyEvent(d, c, 1, 0)
    if hold:
        x11.XFlush(d)
        time.sleep(hold)
    for c in reversed(codes):
        xtst.XTestFakeKeyEvent(d, c, 0, 0)
    x11.XFlush(d)
    x11.XCloseDisplay(d)


def move(x: int, y: int) -> None:
    x11, d = open_display()
    x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
    x11.XDefaultRootWindow.restype = ctypes.c_ulong
    x11.XWarpPointer.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong] + [ctypes.c_int] * 2 + [ctypes.c_uint] * 2 + [ctypes.c_int] * 2
    x11.XWarpPointer(d, 0, x11.XDefaultRootWindow(d), 0, 0, 0, 0, x, y)
    x11.XFlush(d)
    x11.XCloseDisplay(d)


def active() -> str:
    out = subprocess.run(["xprop", "-root", "_NET_ACTIVE_WINDOW"], capture_output=True, text=True).stdout
    m = re.search(r"(0x[0-9a-f]+)", out)
    if not m or int(m.group(1), 16) == 0:
        return ""
    name = subprocess.run(["xprop", "-id", m.group(1), "_NET_WM_NAME", "WM_NAME"], capture_output=True, text=True).stdout
    n = re.search(r'= "(.*)"', name)
    return n.group(1) if n else ""


def visible(text: str) -> int:
    tree = subprocess.run(["xwininfo", "-root", "-tree"], capture_output=True, text=True).stdout
    ids = [m.group(1) for m in re.finditer(r'(0x[0-9a-f]+) "([^"]*)"', tree) if text in m.group(2)]
    n = 0
    for i in ids:
        info = subprocess.run(["xwininfo", "-id", i], capture_output=True, text=True).stdout
        if "Map State: IsViewable" in info:
            n += 1
    return n


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "key":
        key(sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else 0)
    elif cmd == "move":
        move(int(sys.argv[2]), int(sys.argv[3]))
    elif cmd == "active":
        print(active())
    elif cmd == "visible":
        print(visible(sys.argv[2]))
    else:
        sys.exit(__doc__)
