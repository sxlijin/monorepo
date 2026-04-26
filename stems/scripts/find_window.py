#!/usr/bin/env -S uv run --quiet --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["pyobjc-framework-Quartz"]
# ///
"""Print the CGWindowID of the on-screen window owned by the given PID."""
import sys
from Quartz import (
    CGWindowListCopyWindowInfo,
    kCGWindowListOptionOnScreenOnly,
    kCGWindowListExcludeDesktopElements,
    kCGNullWindowID,
)

pid = int(sys.argv[1])
opts = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements
for w in CGWindowListCopyWindowInfo(opts, kCGNullWindowID):
    if w.get("kCGWindowOwnerPID") == pid and w.get("kCGWindowLayer", -1) == 0:
        print(w["kCGWindowNumber"])
        sys.exit(0)
sys.exit(2)
