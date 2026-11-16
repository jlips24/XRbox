"""Test the summary runner using tiny subprocesses, never actual product suites."""
from contextlib import redirect_stdout
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import test_all


class TestAllTests(unittest.TestCase):
    def test_ci_keeps_all_suites_and_reuses_rust_coverage_for_godot(self):
        local = test_all.suite_commands('make')
        ci = test_all.suite_commands('make', ci=True)
        self.assertEqual([name for name, _ in local], [name for name, _ in ci])
        self.assertEqual(ci[:2], local[:2])
        self.assertIn('test-rust-all', ci[0][1])
        self.assertIn('xr-check', local[2][1])
        self.assertIn('xr-ui-check', ci[2][1])
        self.assertIn('xr-render', ci[3][1])
        for _, command in ci[2:]:
            self.assertIn('ARGS=--workspace-features', command)

    def test_retains_output_continues_after_failure_and_reports_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            terminal = io.StringIO()
            suites = [
                ('Broken', [sys.executable, '-c', 'import sys; print("failure detail", file=sys.stderr); sys.exit(3)']),
                ('Later', [sys.executable, '-c', 'print("verbose output" * 1000)']),
            ]
            with redirect_stdout(terminal):
                self.assertEqual(test_all.run_suites(suites, directory), 1)
            self.assertIn('FAIL', terminal.getvalue())
            self.assertIn('PASS', terminal.getvalue())
            self.assertNotIn('verbose output', terminal.getvalue())
            self.assertIn('failure detail', (directory / '01-broken.log').read_text())
            self.assertIn('verbose output', (directory / '02-later.log').read_text())

    def test_success_and_launch_failure_exit_codes(self):
        for command, expected in [([sys.executable, '-c', 'pass'], 0),
                                  (['/nonexistent/xrbox-test-command'], 1)]:
            with self.subTest(command=command), tempfile.TemporaryDirectory() as temporary:
                with redirect_stdout(io.StringIO()):
                    self.assertEqual(test_all.run_suites([('Suite', command)], Path(temporary)), expected)

    def test_interruption_reports_remaining_suites_without_starting_them(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(test_all.subprocess, 'run', side_effect=KeyboardInterrupt) as run:
            terminal = io.StringIO()
            with redirect_stdout(terminal):
                self.assertEqual(test_all.run_suites([('First', ['fake']), ('Next', ['fake'])], Path(temporary)), 130)
            self.assertEqual(run.call_count, 1)
            self.assertIn('INTERRUPTED', terminal.getvalue())
            self.assertIn('NOT RUN', terminal.getvalue())


if __name__ == '__main__':
    unittest.main()
