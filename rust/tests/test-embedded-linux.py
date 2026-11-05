#!/usr/bin/env python3
"""Exercise the packaged embedded_linux example in an already-booted QEMU guest."""

import argparse
import json
from pathlib import Path
import shlex
import socket
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ssh-host", default="127.0.0.1")
    parser.add_argument("--ssh-port", type=int, default=22273)
    parser.add_argument("--ssh-user", default="tester")
    parser.add_argument("--identity", required=True, type=Path)
    parser.add_argument("--known-hosts", required=True, type=Path)
    parser.add_argument("--qmp-port", required=True, type=int)
    parser.add_argument("--bundle", required=True, help="Absolute guest bundle directory")
    parser.add_argument("--executable", default="embedded_linux", help="Sample executable basename")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if not args.bundle.startswith("/"):
        parser.error("--bundle must be an absolute guest path")
    if not args.executable or "/" in args.executable or args.executable in (".", ".."):
        parser.error("--executable must be a basename")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    ssh = [
        "ssh", "-p", str(args.ssh_port), "-i", str(args.identity.resolve()),
        "-o", "BatchMode=yes", "-o", "ConnectTimeout=10",
        "-o", "StrictHostKeyChecking=yes",
        "-o", f"UserKnownHostsFile={args.known_hosts.resolve()}",
        f"{args.ssh_user}@{args.ssh_host}",
    ]
    log_path = output / "embedded-linux.log"
    process = None
    with socket.create_connection(("127.0.0.1", args.qmp_port), timeout=10) as connection:
        stream = connection.makefile("rwb")
        greeting = json.loads(stream.readline())
        if "QMP" not in greeting:
            raise RuntimeError(f"Unexpected QMP greeting: {greeting}")

        def qmp(command, arguments=None):
            request = {"execute": command}
            if arguments is not None:
                request["arguments"] = arguments
            stream.write(json.dumps(request).encode() + b"\n")
            stream.flush()
            while True:
                line = stream.readline()
                if not line:
                    raise RuntimeError("QMP disconnected")
                response = json.loads(line)
                if "error" in response:
                    raise RuntimeError(response["error"])
                if "return" in response:
                    return response["return"]

        def wait_for(marker, start=0):
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                text = log_path.read_text()[start:]
                if marker in text:
                    return
                if process.poll() is not None:
                    raise RuntimeError(f"Application exited before {marker!r}:\n{text}")
                time.sleep(0.1)
            raise RuntimeError(f"Timed out waiting for {marker!r}:\n{log_path.read_text()}")

        def screenshot(name):
            path = output / name
            qmp("screendump", {"filename": str(path)})
            return path.read_bytes()

        def click(x, y):
            qmp("input-send-event", {"events": [
                {"type": "abs", "data": {"axis": "x", "value": round(x * 32767)}},
                {"type": "abs", "data": {"axis": "y", "value": round(y * 32767)}},
            ]})
            qmp("input-send-event", {"events": [
                {"type": "btn", "data": {"button": "left", "down": True}},
            ]})
            time.sleep(0.1)
            qmp("input-send-event", {"events": [
                {"type": "btn", "data": {"button": "left", "down": False}},
            ]})

        qmp("qmp_capabilities")
        with log_path.open("w") as log:
            try:
                command = (
                    f"cd {shlex.quote(args.bundle)} && "
                    'echo "rustolonia-test-pid=$$" && '
                    "exec env -u DISPLAY -u WAYLAND_DISPLAY -u RUSTOLONIA_HOST_LIB "
                    "-u RUSTOLONIA_DRM_SCALING -u RUSTOLONIA_DRM_ROTATION "
                    f"./{shlex.quote(args.executable)}"
                )
                process = subprocess.Popen(ssh + [command], stdout=log, stderr=subprocess.STDOUT)
                wait_for("embedded-linux: ready")
                # The ready marker precedes presentation. Wait for a nonblank frame.
                deadline = time.monotonic() + 30
                while True:
                    before = screenshot("embedded-before.ppm")
                    header, dimensions, maximum, pixels = before.split(b"\n", 3)
                    if header != b"P6" or dimensions != b"1280 800" or maximum != b"255":
                        raise RuntimeError("This fixture requires a 1280x800 QEMU display")
                    background = bytes((235, 240, 248))
                    if pixels.count(background) > 100_000:
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError("The application's background was not presented")
                    time.sleep(0.1)
                click(0.5, 0.515)
                wait_for("embedded-linux: count=1")
                deadline = time.monotonic() + 10
                while True:
                    after = screenshot("embedded-after.ppm")
                    after_pixels = after.split(b"\n", 3)[3]
                    # Check the counter, not the pointer or the button's hover state.
                    changed = sum(
                        a != b
                        for row in range(305, 350)
                        for a, b in zip(
                            pixels[(row * 1280 + 380) * 3:(row * 1280 + 660) * 3],
                            after_pixels[(row * 1280 + 380) * 3:(row * 1280 + 660) * 3],
                        )
                    )
                    if changed > 100:
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError("Counter event did not produce a changed display")
                    time.sleep(0.1)
                click(0.5, 0.645)
                wait_for("embedded-linux: stopped")
                if process.wait(timeout=10) != 0:
                    raise RuntimeError(f"Application failed:\n{log_path.read_text()}")

                restart_start = log.seek(0, 2)
                process = subprocess.Popen(ssh + [command], stdout=log, stderr=subprocess.STDOUT)
                wait_for("embedded-linux: ready", restart_start)
                text = log_path.read_text()[restart_start:]
                pid = next(int(line.split("=", 1)[1]) for line in text.splitlines()
                           if line.startswith("rustolonia-test-pid="))
                subprocess.run(ssh + [f"kill -TERM {pid}"], check=True, timeout=15)
                wait_for("embedded-linux: stopped", restart_start)
                if process.wait(timeout=10) != 0:
                    raise RuntimeError("SIGTERM did not shut down cleanly")
                print("Passed: DRM presentation, pointer input, counter update, button shutdown, restart, SIGTERM.")
            finally:
                if process is not None and process.poll() is None:
                    text = log_path.read_text()
                    pids = [int(line.split("=", 1)[1]) for line in text.splitlines()
                            if line.startswith("rustolonia-test-pid=")]
                    if pids:
                        subprocess.run(ssh + [f"kill -TERM {pids[-1]}"], timeout=15, check=False)
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        if pids:
                            subprocess.run(ssh + [f"kill -KILL {pids[-1]}"], timeout=15, check=False)
                        process.terminate()
                        process.wait(timeout=10)


if __name__ == "__main__":
    main()
