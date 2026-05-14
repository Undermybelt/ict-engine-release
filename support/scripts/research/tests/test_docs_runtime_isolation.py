from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parents[4]


class DocsRuntimeIsolationTests(unittest.TestCase):
    def test_guard_script_passes_when_runtime_sources_do_not_reference_docs_plans(self) -> None:
        script = REPO_ROOT / "support" / "scripts" / "ci" / "check_docs_runtime_isolation.py"
        result = subprocess.run(
            [
                sys.executable,
                str(script),
                "--root",
                str(REPO_ROOT),
                "--extra-file",
                str(REPO_ROOT / "support" / "scripts" / "research" / "factor_candidate_resolver.py"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertIn("docs runtime isolation ok", result.stdout)

    def test_guard_script_rejects_docs_plan_references_in_extra_file(self) -> None:
        script = REPO_ROOT / "support" / "scripts" / "ci" / "check_docs_runtime_isolation.py"
        with TemporaryDirectory() as tmpdir:
            bad_file = Path(tmpdir) / "bad_runtime.py"
            forbidden_ref = "support/docs" + "/plans/example-plan.md"
            bad_file.write_text(
                f"PLAN = '{forbidden_ref}'\n",
                encoding="utf-8",
            )

            result = subprocess.run(
                [
                    sys.executable,
                    str(script),
                    "--root",
                    str(REPO_ROOT),
                    "--extra-file",
                    str(bad_file),
                ],
                capture_output=True,
                text=True,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(forbidden_ref, result.stdout)


if __name__ == "__main__":
    unittest.main()
