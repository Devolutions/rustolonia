#!/usr/bin/env python3
"""Retry a release browser-host job after a confirmed runner shutdown."""

import json
import os
from pathlib import Path
import subprocess
import sys
import time


WORKFLOW_PATH = ".github/workflows/release.yml"
JOB_NAME = "browser host"
SHUTDOWN_MARKER = "##[error]The runner has received a shutdown signal."
API_TIMEOUT_SECONDS = 60


def github_api(repository, endpoint, *flags):
    command = ["gh", "api", *flags, f"repos/{repository}/{endpoint}"]
    delays = () if "--method" in flags else (2, 5)
    for attempt in range(len(delays) + 1):
        try:
            result = subprocess.run(
                command, capture_output=True, text=True, check=False, timeout=API_TIMEOUT_SECONDS
            )
            if result.returncode == 0:
                return result.stdout
            detail = result.stderr.strip() or f"exit status {result.returncode}"
        except subprocess.TimeoutExpired:
            detail = f"timed out after {API_TIMEOUT_SECONDS}s"
        if attempt == len(delays):
            raise RuntimeError(f"GitHub API request for {endpoint} failed: {detail}")
        delay = delays[attempt]
        print(
            f"GitHub API request for {endpoint} failed: {detail}; retrying in {delay}s.",
            file=sys.stderr,
        )
        time.sleep(delay)


def is_release_dispatch(run, repository):
    return (
        run["path"] == WORKFLOW_PATH
        and run["event"] == "workflow_dispatch"
        and run["head_branch"] == "master"
        and run["repository"]["full_name"] == repository
        and run["head_repository"]["full_name"] == repository
    )


def is_original_failure(run):
    return (
        run["run_attempt"] == 1
        and run["status"] == "completed"
        and run["conclusion"] == "failure"
    )


def interrupted_job(job):
    if job["status"] != "completed" or job["conclusion"] != "failure":
        return False
    conclusions = [step["conclusion"] for step in job["steps"]]
    return (
        "cancelled" in conclusions
        and all(value in {"success", "skipped", "cancelled"} for value in conclusions)
    )


def retry(event, repository, api=github_api):
    run = event["workflow_run"]
    if (
        event["action"] != "completed"
        or event["repository"]["full_name"] != repository
        or not is_release_dispatch(run, repository)
        or not is_original_failure(run)
    ):
        print("Not a failed first-attempt Release dispatch from master; not retrying.")
        return

    run_id = run["id"]
    if type(run_id) is not int or run_id <= 0:
        raise RuntimeError("The workflow run ID is invalid.")

    def current_run():
        current = json.loads(api(repository, f"actions/runs/{run_id}"))
        if (
            current["id"] != run_id
            or current["head_sha"] != run["head_sha"]
            or not is_release_dispatch(current, repository)
        ):
            raise RuntimeError("The live workflow run does not match the completed event.")
        return current

    if not is_original_failure(current_run()):
        print(f"Release run {run_id} has already changed; not retrying.")
        return

    pages = json.loads(api(
        repository, f"actions/runs/{run_id}/attempts/1/jobs?per_page=100",
        "--paginate", "--slurp",
    ))
    jobs = [job for page in pages for job in page["jobs"] if job["name"] == JOB_NAME]
    if not jobs:
        print("The failed release has no browser host job; not retrying.")
        return
    if len(jobs) != 1:
        raise RuntimeError("The release has more than one browser host job in its first attempt.")
    job = jobs[0]
    if job["run_id"] != run_id or job["run_attempt"] != 1:
        raise RuntimeError("The browser host job does not belong to the original run attempt.")
    if not interrupted_job(job):
        print("The browser host did not have an isolated interrupted step; not retrying.")
        return

    job_id = job["id"]
    if type(job_id) is not int or job_id <= 0:
        raise RuntimeError("The browser host job ID is invalid.")
    log = api(repository, f"actions/jobs/{job_id}/logs", "--allow-escape-sequences")
    if SHUTDOWN_MARKER not in log:
        print("The browser host log has no runner shutdown signal; not retrying.")
        return

    if not is_original_failure(current_run()):
        print(f"Release run {run_id} was already re-run or cancelled; not retrying.")
        return
    try:
        api(repository, f"actions/jobs/{job_id}/rerun", "--method", "POST", "--silent")
    except RuntimeError as error:
        print(f"Rerun request failed: {error}; checking the live run state.", file=sys.stderr)
        if not is_original_failure(current_run()):
            print(f"Release run {run_id} was already re-run or cancelled; not retrying again.")
            return
        raise
    print(f"Requested one fresh-runner retry of browser host job {job_id} in release run {run_id}.")


def main():
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    try:
        retry(event, os.environ["GITHUB_REPOSITORY"])
    except RuntimeError as error:
        print(f"::error::{error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
