#!/usr/bin/env python3
"""Exercise the release browser-host retry without changing GitHub runs."""

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


sys.dont_write_bytecode = True
SCRIPT = Path(__file__).resolve().parents[2] / ".github/scripts/retry-interrupted-browser.py"
ROOT = SCRIPT.parents[2]
spec = importlib.util.spec_from_file_location("browser_retry", SCRIPT)
browser_retry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(browser_retry)

REPOSITORY = "Devolutions/rustolonia"
RUN_ID = 12345
JOB_ID = 67890


def release_run():
    return {
        "id": RUN_ID,
        "path": ".github/workflows/release.yml",
        "event": "workflow_dispatch",
        "head_branch": "master",
        "head_sha": "a" * 40,
        "head_repository": {"full_name": REPOSITORY},
        "repository": {"full_name": REPOSITORY},
        "run_attempt": 1,
        "status": "completed",
        "conclusion": "failure",
    }


def completed_event():
    return {
        "action": "completed",
        "repository": {"full_name": REPOSITORY},
        "workflow_run": release_run(),
    }


def browser_job():
    return {
        "id": JOB_ID,
        "run_id": RUN_ID,
        "run_attempt": 1,
        "name": "browser host",
        "status": "completed",
        "conclusion": "failure",
        "steps": [
            {"name": "Set up job", "conclusion": "success"},
            {"name": "Package the browser host", "conclusion": "cancelled"},
            {"name": "Upload artifact", "conclusion": "skipped"},
        ],
    }


class FakeGitHub:
    def __init__(self):
        self.calls = []
        self.runs = [release_run()]
        self.pages = [{"jobs": [browser_job()]}]
        self.log = "2026-11-06T15:38:08Z " + browser_retry.SHUTDOWN_MARKER
        self.post_error = None

    def __call__(self, repository, endpoint, *flags):
        self.calls.append((repository, endpoint, flags))
        if endpoint == f"actions/runs/{RUN_ID}":
            run = self.runs.pop(0) if len(self.runs) > 1 else self.runs[0]
            return json.dumps(run)
        if endpoint == f"actions/runs/{RUN_ID}/attempts/1/jobs?per_page=100":
            return json.dumps(self.pages)
        if endpoint == f"actions/jobs/{JOB_ID}/logs":
            return self.log
        if endpoint == f"actions/jobs/{JOB_ID}/rerun":
            if self.post_error:
                raise RuntimeError(self.post_error)
            return ""
        raise AssertionError(f"Unexpected API request: {endpoint}")

    def posts(self):
        return [call for call in self.calls if call[1].endswith("/rerun")]


class RetryTests(unittest.TestCase):
    def setUp(self):
        self.event = completed_event()
        self.api = FakeGitHub()
        self.output = io.StringIO()
        self.error = io.StringIO()

    def retry(self):
        with (contextlib.redirect_stdout(self.output),
              contextlib.redirect_stderr(self.error)):
            browser_retry.retry(self.event, REPOSITORY, self.api)

    def test_confirmed_shutdown_retries_only_the_original_browser_job(self):
        self.retry()
        self.assertEqual(self.api.posts(), [(
            REPOSITORY, f"actions/jobs/{JOB_ID}/rerun", ("--method", "POST", "--silent")
        )])
        self.assertIn((
            REPOSITORY, f"actions/runs/{RUN_ID}/attempts/1/jobs?per_page=100",
            ("--paginate", "--slurp"),
        ), self.api.calls)
        self.assertIn((
            REPOSITORY, f"actions/jobs/{JOB_ID}/logs", ("--allow-escape-sequences",)
        ), self.api.calls)
        self.assertIn("one fresh-runner retry", self.output.getvalue())

    def test_browser_job_is_found_on_later_pages(self):
        other = dict(browser_job(), id=1, name="Android host")
        self.api.pages = [{"jobs": [other]}, {"jobs": [browser_job()]}]
        self.retry()
        self.assertEqual(len(self.api.posts()), 1)

    def test_cancelled_step_without_shutdown_is_not_retried(self):
        self.api.log = "##[error]The operation was canceled."
        self.retry()
        self.assertFalse(self.api.posts())
        self.assertIn("no runner shutdown signal", self.output.getvalue())

    def test_regular_failure_before_shutdown_is_not_retried(self):
        self.api.pages[0]["jobs"][0]["steps"].insert(
            1, {"name": "Build", "conclusion": "failure"}
        )
        self.retry()
        self.assertFalse(self.api.posts())
        self.assertFalse(any(call[1].endswith("/logs") for call in self.api.calls))

    def test_cancelled_job_and_unknown_step_state_are_not_retried(self):
        for change in (
            lambda job: job.update(conclusion="cancelled"),
            lambda job: job["steps"][0].update(conclusion=None),
            lambda job: job["steps"][1].update(conclusion="failure"),
        ):
            with self.subTest(change=change):
                self.api = FakeGitHub()
                change(self.api.pages[0]["jobs"][0])
                self.retry()
                self.assertFalse(self.api.posts())

    def test_other_job_does_not_trigger_a_retry(self):
        self.api.pages[0]["jobs"][0]["name"] = "Android host"
        self.retry()
        self.assertFalse(self.api.posts())
        self.assertIn("no browser host job", self.output.getvalue())

    def test_duplicate_browser_jobs_fail_closed(self):
        self.api.pages.append({"jobs": [browser_job()]})
        with self.assertRaisesRegex(RuntimeError, "more than one browser host job"):
            self.retry()
        self.assertFalse(self.api.posts())

    def test_wrong_job_identity_fails_closed(self):
        self.api.pages[0]["jobs"][0]["run_id"] = RUN_ID + 1
        with self.assertRaisesRegex(RuntimeError, "original run attempt"):
            self.retry()
        self.assertFalse(self.api.posts())

    def test_unrelated_or_cancelled_events_do_not_call_github(self):
        mutations = (
            lambda event: event.update(action="requested"),
            lambda event: event["repository"].update(full_name="another/repo"),
            lambda event: event["workflow_run"].update(conclusion="cancelled"),
            lambda event: event["workflow_run"].update(run_attempt=2),
            lambda event: event["workflow_run"].update(path=".github/workflows/other.yml"),
            lambda event: event["workflow_run"].update(event="push"),
            lambda event: event["workflow_run"].update(head_branch="feature"),
            lambda event: event["workflow_run"]["head_repository"].update(full_name="fork/repo"),
            lambda event: event["workflow_run"]["repository"].update(full_name="fork/repo"),
        )
        for change in mutations:
            with self.subTest(change=change):
                self.event = completed_event()
                self.api = FakeGitHub()
                change(self.event)
                self.retry()
                self.assertEqual(self.api.calls, [])

    def test_already_rerun_before_inspection_is_not_retried(self):
        self.api.runs[0].update(run_attempt=2, status="in_progress", conclusion=None)
        self.retry()
        self.assertEqual(len(self.api.calls), 1)
        self.assertFalse(self.api.posts())

    def test_rerun_started_while_inspecting_is_not_duplicated(self):
        changed = release_run()
        changed.update(run_attempt=2, status="in_progress", conclusion=None)
        self.api.runs = [release_run(), changed]
        self.retry()
        self.assertFalse(self.api.posts())

    def test_mismatched_live_run_fails_closed(self):
        self.api.runs[0]["head_sha"] = "b" * 40
        with self.assertRaisesRegex(RuntimeError, "does not match"):
            self.retry()
        self.assertFalse(self.api.posts())

    def test_failed_post_is_reported_without_repeating_it(self):
        self.api.post_error = "permission denied"
        with self.assertRaisesRegex(RuntimeError, "permission denied"):
            self.retry()
        self.assertEqual(len(self.api.posts()), 1)
        self.assertIn("permission denied", self.error.getvalue())

    def test_failed_post_with_a_concurrent_rerun_is_not_repeated(self):
        changed = release_run()
        changed.update(run_attempt=2, status="in_progress", conclusion=None)
        self.api.runs = [release_run(), release_run(), changed]
        self.api.post_error = "already in progress"
        self.retry()
        self.assertEqual(len(self.api.posts()), 1)
        self.assertIn("not retrying again", self.output.getvalue())

    def test_invalid_run_and_job_ids_fail_closed(self):
        self.event["workflow_run"]["id"] = True
        with self.assertRaisesRegex(RuntimeError, "run ID is invalid"):
            self.retry()
        self.event = completed_event()
        self.api.pages[0]["jobs"][0]["id"] = 0
        with self.assertRaisesRegex(RuntimeError, "job ID is invalid"):
            self.retry()
        self.assertFalse(self.api.posts())


class GitHubApiTests(unittest.TestCase):
    def test_calls_gh_with_separate_arguments(self):
        result = subprocess.CompletedProcess([], 0, stdout="log", stderr="")
        with mock.patch.object(browser_retry.subprocess, "run", return_value=result) as run:
            value = browser_retry.github_api(
                REPOSITORY, f"actions/jobs/{JOB_ID}/logs", "--allow-escape-sequences"
            )
        self.assertEqual(value, "log")
        run.assert_called_once_with(
            ["gh", "api", "--allow-escape-sequences",
             f"repos/{REPOSITORY}/actions/jobs/{JOB_ID}/logs"],
            capture_output=True, text=True, check=False,
            timeout=browser_retry.API_TIMEOUT_SECONDS,
        )

    def test_api_errors_include_the_diagnostic(self):
        result = subprocess.CompletedProcess([], 1, stdout="", stderr="HTTP 403")
        with (mock.patch.object(browser_retry.subprocess, "run", return_value=result) as run,
              mock.patch.object(browser_retry.time, "sleep"),
              contextlib.redirect_stderr(io.StringIO())):
            with self.assertRaisesRegex(RuntimeError, "HTTP 403"):
                browser_retry.github_api(REPOSITORY, "actions/runs/123")
        self.assertEqual(run.call_count, 3)

    def test_transient_read_errors_are_retried(self):
        failed = subprocess.CompletedProcess([], 1, stdout="", stderr="HTTP 503")
        success = subprocess.CompletedProcess([], 0, stdout="{}", stderr="")
        with (mock.patch.object(browser_retry.subprocess, "run", side_effect=[failed, success]) as run,
              mock.patch.object(browser_retry.time, "sleep") as sleep,
              contextlib.redirect_stderr(io.StringIO())):
            self.assertEqual(browser_retry.github_api(REPOSITORY, "actions/runs/123"), "{}")
        self.assertEqual(run.call_count, 2)
        sleep.assert_called_once_with(2)

    def test_post_errors_are_not_blindly_retried(self):
        result = subprocess.CompletedProcess([], 1, stdout="", stderr="HTTP 503")
        with (mock.patch.object(browser_retry.subprocess, "run", return_value=result) as run,
              mock.patch.object(browser_retry.time, "sleep") as sleep):
            with self.assertRaisesRegex(RuntimeError, "HTTP 503"):
                browser_retry.github_api(REPOSITORY, "actions/jobs/123/rerun", "--method", "POST")
        self.assertEqual(run.call_count, 1)
        sleep.assert_not_called()

    def test_api_timeout_is_bounded_and_reported(self):
        timeout = subprocess.TimeoutExpired("gh", browser_retry.API_TIMEOUT_SECONDS)
        with (mock.patch.object(browser_retry.subprocess, "run", side_effect=timeout) as run,
              mock.patch.object(browser_retry.time, "sleep"),
              contextlib.redirect_stderr(io.StringIO())):
            with self.assertRaisesRegex(RuntimeError, "timed out after 60s"):
                browser_retry.github_api(REPOSITORY, "actions/runs/123")
        self.assertEqual(run.call_count, 3)

    def test_post_timeout_is_not_blindly_retried(self):
        timeout = subprocess.TimeoutExpired("gh", browser_retry.API_TIMEOUT_SECONDS)
        with (mock.patch.object(browser_retry.subprocess, "run", side_effect=timeout) as run,
              mock.patch.object(browser_retry.time, "sleep") as sleep):
            with self.assertRaisesRegex(RuntimeError, "timed out after 60s"):
                browser_retry.github_api(REPOSITORY, "actions/jobs/123/rerun", "--method", "POST")
        self.assertEqual(run.call_count, 1)
        sleep.assert_not_called()


class EntrypointTests(unittest.TestCase):
    def run_main(self, api):
        def gh(command, **kwargs):
            self.assertEqual(command[:2], ["gh", "api"])
            prefix = f"repos/{REPOSITORY}/"
            self.assertTrue(command[-1].startswith(prefix))
            endpoint = command[-1][len(prefix):]
            try:
                output = api(REPOSITORY, endpoint, *command[2:-1])
                return subprocess.CompletedProcess(command, 0, stdout=output, stderr="")
            except RuntimeError as error:
                return subprocess.CompletedProcess(command, 1, stdout="", stderr=str(error))

        with tempfile.TemporaryDirectory() as directory:
            event_path = Path(directory) / "event.json"
            event_path.write_text(json.dumps(completed_event()), encoding="utf-8")
            environment = {
                "GITHUB_EVENT_PATH": str(event_path),
                "GITHUB_REPOSITORY": REPOSITORY,
            }
            output, error = io.StringIO(), io.StringIO()
            with (mock.patch.dict(os.environ, environment),
                  mock.patch.object(browser_retry.subprocess, "run", side_effect=gh),
                  contextlib.redirect_stdout(output), contextlib.redirect_stderr(error)):
                code = browser_retry.main()
            return code, output.getvalue(), error.getvalue()

    def test_event_entrypoint_submits_one_targeted_rerun(self):
        api = FakeGitHub()
        code, output, error = self.run_main(api)
        self.assertEqual(code, 0)
        self.assertEqual(len(api.posts()), 1)
        self.assertIn("fresh-runner retry", output)
        self.assertEqual(error, "")

    def test_event_entrypoint_surfaces_api_failure(self):
        api = FakeGitHub()
        api.post_error = "HTTP 403"
        code, _, error = self.run_main(api)
        self.assertEqual(code, 1)
        self.assertEqual(len(api.posts()), 1)
        self.assertIn("::error::", error)
        self.assertIn("HTTP 403", error)


class WorkflowContractTests(unittest.TestCase):
    def job_block(self, workflow, job):
        match = re.search(
            rf"(?ms)^  {re.escape(job)}:\n(.*?)(?=^  [\w-]+:\n|\Z)", workflow
        )
        self.assertIsNotNone(match, f"Missing job {job}")
        return match.group(1)

    def test_release_job_identity_and_retry_safe_artifact(self):
        release = (ROOT / browser_retry.WORKFLOW_PATH).read_text(encoding="utf-8")
        browser = self.job_block(release, "build-browser")
        self.assertIn(f"    name: {browser_retry.JOB_NAME}\n", browser)
        upload = re.search(
            r"(?ms)^      - uses: actions/upload-artifact@v4\n(.*?)(?=^      - |\Z)",
            browser,
        )
        self.assertIsNotNone(upload)
        self.assertIn("name: rustolonia-host-browser-wasm", upload.group(1))
        self.assertIn("overwrite: true", upload.group(1))
        stage = self.job_block(release, "stage-release")
        self.assertIn("pattern: rustolonia-host-*", stage)

    def test_monitor_uses_the_release_workflow_and_retry_script(self):
        release = (ROOT / browser_retry.WORKFLOW_PATH).read_text(encoding="utf-8")
        monitor = (ROOT / ".github/workflows/retry-interrupted-browser.yml").read_text(
            encoding="utf-8"
        )
        self.assertRegex(release, r"(?m)^name: Release$")
        self.assertIn("workflows: [Release]", monitor)
        self.assertIn("types: [completed]", monitor)
        self.assertIn("branches: [master]", monitor)
        self.assertIn("github.event.workflow_run.run_attempt == 1", monitor)
        retry = self.job_block(monitor, "retry")
        self.assertIn("timeout-minutes: 10", retry)
        self.assertIn("actions: write", retry)
        self.assertIn("contents: read", retry)
        self.assertIn("ref: ${{ github.sha }}", retry)
        self.assertIn("persist-credentials: false", retry)
        self.assertIn("python3 .github/scripts/retry-interrupted-browser.py", monitor)


if __name__ == "__main__":
    unittest.main()
