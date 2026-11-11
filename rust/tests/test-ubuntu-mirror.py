#!/usr/bin/env python3
"""Check Ubuntu APT source rewriting without changing the host's configuration."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "use-ubuntu-mirror.sh"


class UbuntuMirrorTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        self.apt = root / "apt"
        (self.apt / "sources.list.d").mkdir(parents=True)
        binary_dir = root / "bin"
        binary_dir.mkdir()
        dpkg = binary_dir / "dpkg"
        dpkg.write_text("#!/bin/sh\nprintf '%s\\n' \"$TEST_DPKG_ARCH\"\n")
        dpkg.chmod(0o755)
        self.env = os.environ.copy()
        self.env["PATH"] = f"{binary_dir}{os.pathsep}{self.env['PATH']}"

    def run_helper(self, architecture, check=True):
        env = self.env.copy()
        env["TEST_DPKG_ARCH"] = architecture
        return subprocess.run(
            [str(SCRIPT), str(self.apt)], env=env, check=check,
            capture_output=True, text=True,
        )

    def test_amd64_rewrites_mirrorlists_and_ubuntu_sources(self):
        sources = self.apt / "sources.list"
        sources.write_text(
            "deb mirror+file:/etc/apt/apt-mirrors.txt jammy main\n"
            "deb http://azure.archive.ubuntu.com/ubuntu/ jammy-updates main\n"
            "deb https://packages.microsoft.com/ubuntu/22.04/prod jammy main\n"
            "deb mirror+file:/etc/apt/apt-mirrors.txt.other stable main\n"
        )
        deb822 = self.apt / "sources.list.d" / "ubuntu.sources"
        deb822.write_text(
            "Types: deb\n"
            "URIs: mirror+file:///etc/apt/apt-mirrors.txt\n"
            "Suites: jammy jammy-updates jammy-security\n"
            "Components: main universe\n"
        )
        legacy = self.apt / "sources.list.d" / "security.list"
        legacy.write_text("deb https://security.ubuntu.com/ubuntu jammy-security main\n")

        self.run_helper("amd64")

        self.assertEqual(
            sources.read_text(),
            "deb https://mirrors.mit.edu/ubuntu/ jammy main\n"
            "deb https://mirrors.mit.edu/ubuntu/ jammy-updates main\n"
            "deb https://packages.microsoft.com/ubuntu/22.04/prod jammy main\n"
            "deb mirror+file:/etc/apt/apt-mirrors.txt.other stable main\n",
        )
        self.assertEqual(
            deb822.read_text(),
            "Types: deb\n"
            "URIs: https://mirrors.mit.edu/ubuntu/\n"
            "Suites: jammy jammy-updates jammy-security\n"
            "Components: main universe\n",
        )
        self.assertEqual(
            legacy.read_text(),
            "deb https://mirrors.mit.edu/ubuntu/ jammy-security main\n",
        )
        before = [path.read_text() for path in (sources, deb822, legacy)]
        self.run_helper("amd64")
        self.assertEqual(before, [path.read_text() for path in (sources, deb822, legacy)])

    def test_arm64_uses_the_ports_mirror(self):
        sources = self.apt / "sources.list"
        sources.write_text(
            "deb mirror+file:/etc/apt/apt-mirrors.txt jammy main\n"
            "deb http://ports.ubuntu.com/ubuntu-ports/ jammy-updates main\n"
        )

        self.run_helper("arm64")

        self.assertEqual(
            sources.read_text(),
            "deb https://mirrors.mit.edu/ubuntu-ports/ jammy main\n"
            "deb https://mirrors.mit.edu/ubuntu-ports/ jammy-updates main\n",
        )

    def test_accepts_an_already_configured_mirror_without_a_trailing_slash(self):
        sources = self.apt / "sources.list"
        original = "deb https://mirrors.mit.edu/ubuntu jammy main\n"
        sources.write_text(original)

        self.run_helper("amd64")

        self.assertEqual(sources.read_text(), original)

    def test_missing_ubuntu_sources_are_reported(self):
        sources = self.apt / "sources.list"
        original = "deb https://packages.microsoft.com/ubuntu/22.04/prod jammy main\n"
        sources.write_text(original)

        result = self.run_helper("amd64", check=False)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("No supported Ubuntu APT sources found", result.stderr)
        self.assertEqual(sources.read_text(), original)

    def test_unsupported_architecture_fails_without_modifying_sources(self):
        sources = self.apt / "sources.list"
        original = "deb http://archive.ubuntu.com/ubuntu/ jammy main\n"
        sources.write_text(original)

        result = self.run_helper("riscv64", check=False)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unsupported Ubuntu architecture: riscv64", result.stderr)
        self.assertEqual(sources.read_text(), original)


if __name__ == "__main__":
    unittest.main()
