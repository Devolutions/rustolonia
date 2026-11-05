#!/usr/bin/env python3
"""Check browser smoke-test shutdown without .NET, Emscripten or Chrome."""

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).resolve().parents[2] / "samples/browser-standalone/smoke-test.py"
spec = importlib.util.spec_from_file_location("browser_smoke", SCRIPT)
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)

# Model a browser child that keeps stderr open and writes to Default after
# its parent exits. Killing just the parent must not count as shutdown.
FAKE_CHROME = r"""
import os
from pathlib import Path
import signal
import sys
import time

profile = Path(sys.argv[1]) / "Default"
profile.mkdir()
ready = profile / "ready"
if os.fork() == 0:
    def stop(signum, frame):
        print("CONSOLE: profile writer stopped", file=sys.stderr, flush=True)
        os._exit(0)
    signal.signal(signal.SIGTERM, stop)
    ready.touch()
    while True:
        (profile / "preferences").write_text(str(time.monotonic()))
        time.sleep(0.01)
else:
    while not ready.exists():
        time.sleep(0.01)
    if sys.argv[2] == "ready":
        print("CONSOLE: rustolonia-browser-standalone: main view ready",
              file=sys.stderr, flush=True)
    else:
        print("CONSOLE: intentional startup failure", file=sys.stderr, flush=True)
    if sys.argv[2] == "exit":
        sys.exit(42)
    time.sleep(60)
"""


class ShutdownTests(unittest.TestCase):
    def setUp(self):
        self.chrome = mock.Mock(pid=12345)
        self.chrome.poll.return_value = None
        self.reader = mock.Mock()
        self.reader.is_alive.return_value = False
        self.os = mock.Mock(name="posix_os")
        self.os.name = "posix"
        patch = mock.patch.object(smoke, "os", self.os)
        patch.start()
        self.addCleanup(patch.stop)

    def test_gracefully_stops_group_and_drains_stderr(self):
        smoke.stop_chrome(self.chrome, self.reader)
        self.os.killpg.assert_called_once_with(self.chrome.pid, signal.SIGTERM)
        self.chrome.wait.assert_called_once_with(timeout=10)
        self.reader.join.assert_called_once_with(timeout=10)
        self.chrome.stderr.close.assert_called_once()

    def test_kills_group_when_browser_ignores_termination(self):
        self.chrome.wait.side_effect = [subprocess.TimeoutExpired("chrome", 10), 0]
        smoke.stop_chrome(self.chrome, self.reader)
        self.assertEqual(self.os.killpg.call_args_list, [
            mock.call(self.chrome.pid, signal.SIGTERM),
            mock.call(self.chrome.pid, signal.SIGKILL),
        ])
        self.chrome.stderr.close.assert_called_once()

    def test_kills_remaining_children_after_parent_exits(self):
        self.chrome.poll.return_value = 0
        self.reader.is_alive.side_effect = [True, False]
        smoke.stop_chrome(self.chrome, self.reader)
        self.assertEqual(self.os.killpg.call_args_list, [
            mock.call(self.chrome.pid, signal.SIGTERM),
            mock.call(self.chrome.pid, signal.SIGKILL),
        ])
        self.assertEqual(self.reader.join.call_count, 2)

    def test_already_exited_group_is_reaped(self):
        self.os.killpg.side_effect = ProcessLookupError
        smoke.stop_chrome(self.chrome, self.reader)
        self.chrome.wait.assert_called_once_with(timeout=10)
        self.chrome.stderr.close.assert_called_once()

    def test_shutdown_failure_is_not_ignored(self):
        self.reader.is_alive.return_value = True
        with self.assertRaisesRegex(SystemExit, "child processes"):
            smoke.stop_chrome(self.chrome, self.reader)
        self.chrome.stderr.close.assert_not_called()

    def test_windows_stops_entire_process_tree(self):
        self.os.name = "nt"
        with mock.patch.object(smoke.subprocess, "run") as run:
            smoke.stop_chrome(self.chrome, self.reader)
        run.assert_called_once_with(
            ["taskkill", "/PID", str(self.chrome.pid), "/T", "/F"],
            check=True, stdout=subprocess.DEVNULL, timeout=10)
        self.os.killpg.assert_not_called()
        self.chrome.stderr.close.assert_called_once()


@unittest.skipUnless(os.name == "posix", "the process-tree fixture uses fork")
class LifecycleTests(unittest.TestCase):
    def exercise(self, mode):
        processes, profiles, servers = [], [], []
        real_popen = subprocess.Popen
        real_server = smoke.http.server.ThreadingHTTPServer

        def start(command, **kwargs):
            profile = next(arg.split("=", 1)[1] for arg in command
                           if arg.startswith("--user-data-dir="))
            profiles.append(Path(profile))
            # Isolate the fixture even against the old implementation so the
            # test can clean up the descendants it deliberately leaves alive.
            isolated = kwargs.get("start_new_session")
            kwargs["start_new_session"] = True
            process = real_popen(
                [sys.executable, "-u", "-c", FAKE_CHROME, profile, mode], **kwargs)
            processes.append((process, isolated))
            return process

        def serve(*args, **kwargs):
            server = real_server(*args, **kwargs)
            servers.append(server)
            return server

        output = io.StringIO()
        try:
            with tempfile.TemporaryDirectory() as site:
                (Path(site) / "index.html").write_text("<!doctype html>")
                with (mock.patch.object(smoke, "find_chrome", return_value="fake-chrome"),
                      mock.patch.object(smoke.subprocess, "Popen", side_effect=start),
                      mock.patch.object(smoke.http.server, "ThreadingHTTPServer", side_effect=serve),
                      mock.patch.object(sys, "argv", [str(SCRIPT), site, "--timeout", "1"]),
                      contextlib.redirect_stdout(output)):
                    if mode == "ready":
                        self.assertEqual(smoke.main(), 0)
                    else:
                        with self.assertRaisesRegex(SystemExit, "did not report its main view"):
                            smoke.main()
            self.assertIn("profile writer stopped", output.getvalue())
            self.assertFalse(profiles[0].exists())
            self.assertTrue(processes[0][1], "Chrome must have its own process group")
            self.assertEqual(servers[0].socket.fileno(), -1)
            self.assertTrue(processes[0][0].stderr.closed)
            return output.getvalue()
        finally:
            for process, _ in processes:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait(timeout=10)
            for server in servers:
                server.server_close()

    def test_stops_profile_writer_before_cleanup(self):
        self.assertIn(f"OK: {smoke.MARKER}", self.exercise("ready"))

    def test_timeout_preserves_console_error_and_fails(self):
        output = self.exercise("timeout")
        self.assertIn("intentional startup failure", output)
        self.assertNotIn("OK:", output)

    def test_early_exit_still_stops_children_and_fails(self):
        self.assertNotIn("OK:", self.exercise("exit"))


if __name__ == "__main__":
    unittest.main()
