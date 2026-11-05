#!/usr/bin/env python3
"""Loads a site built by build.py in headless Chrome and waits until the
Rust app reports that its main view is up.

Usage: python3 smoke-test.py <site-dir> [--timeout SECONDS]

Chrome is taken from CHROME, else the first of google-chrome, chromium,
chromium-browser, chrome or msedge found on PATH (or the usual Windows and
macOS install locations). The page's console goes to Chrome's stderr; a
failed .NET startup, Rust panic or missing export never prints the marker.
"""

import argparse
import functools
import http.server
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time

MARKER = "rustolonia-browser-standalone: main view ready"


def find_chrome():
    if os.environ.get("CHROME"):
        return os.environ["CHROME"]
    for name in ("google-chrome", "chromium", "chromium-browser", "chrome", "msedge"):
        found = shutil.which(name)
        if found:
            return found
    for path in (
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    ):
        if Path(path).is_file():
            return path
    raise SystemExit("error: no Chrome found; set CHROME to a Chrome or Chromium executable")


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format, *args):
        pass


def stop_chrome(chrome, reader):
    # Chrome's children can still write to the profile after the parent exits.
    # Stop the whole tree and wait for its inherited stderr handles to close.
    for force in (False, True):
        if os.name == "nt":
            if chrome.poll() is None:
                subprocess.run(
                    ["taskkill", "/PID", str(chrome.pid), "/T", "/F"],
                    check=True, stdout=subprocess.DEVNULL, timeout=10)
        else:
            try:
                os.killpg(chrome.pid, signal.SIGKILL if force else signal.SIGTERM)
            except ProcessLookupError:
                pass
        try:
            chrome.wait(timeout=10)
        except subprocess.TimeoutExpired:
            if force:
                raise
            continue
        reader.join(timeout=10)
        if not reader.is_alive():
            chrome.stderr.close()
            return
    raise SystemExit("error: Chrome's child processes did not close their log stream after shutdown")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("site", type=Path)
    parser.add_argument("--timeout", type=float, default=180)
    args = parser.parse_args()
    if not (args.site / "index.html").is_file():
        raise SystemExit(f"error: {args.site} has no index.html")

    handler = functools.partial(QuietHandler, directory=str(args.site))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    url = f"http://127.0.0.1:{server.server_address[1]}/"

    log = []
    found = threading.Event()
    try:
        with tempfile.TemporaryDirectory(prefix="rustolonia-chrome-") as profile:
            chrome = subprocess.Popen(
                [find_chrome(), "--headless=new", "--no-sandbox", "--no-first-run",
                 "--use-angle=swiftshader", "--enable-unsafe-swiftshader",
                 "--enable-logging=stderr", "--v=0", f"--user-data-dir={profile}", url],
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, errors="replace",
                start_new_session=(os.name != "nt"))

            def read():
                for line in chrome.stderr:
                    if "CONSOLE" in line or MARKER in line:
                        log.append(line.rstrip())
                    if MARKER in line:
                        found.set()

            reader = threading.Thread(target=read, daemon=True)
            reader.start()
            try:
                deadline = time.monotonic() + args.timeout
                while not found.is_set() and chrome.poll() is None and time.monotonic() < deadline:
                    found.wait(0.5)
            finally:
                stop_chrome(chrome, reader)
    finally:
        server.shutdown()
        server.server_close()
        server_thread.join()
        print("\n".join(log[-40:]))

    if not found.is_set():
        raise SystemExit(f"error: the app did not report its main view within {args.timeout:.0f}s")
    print(f"OK: {MARKER}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
