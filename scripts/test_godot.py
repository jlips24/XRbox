"""Build-graph and subprocess-failure regressions; no engine or compiler launched."""
from contextlib import redirect_stderr, redirect_stdout
import io
import subprocess
import unittest
from unittest.mock import patch

import godot


class GodotHelperTests(unittest.TestCase):
    def test_ci_build_matches_full_workspace_test_features(self):
        command = godot.build_command('cargo', 'host', 'host', workspace_features=True)
        for option in ['--locked', '--workspace', '--all-targets', '--all-features']:
            self.assertIn(option, command)
        self.assertNotIn('-p', command)
        self.assertNotIn('--release', command)

    def test_workspace_mode_cannot_pull_host_dependencies_into_android(self):
        with self.assertRaisesRegex(ValueError, 'host checks'):
            godot.build_command('cargo', 'aarch64-linux-android', 'host', workspace_features=True)
        self.assertEqual(godot.build_command('cargo', 'aarch64-linux-android', 'host'),
                         ['cargo', 'build', '--locked', '-p', 'xrbox-godot', '--target', 'aarch64-linux-android'])

    def test_signal_exit_is_reported_without_obscuring_it_as_exit_250(self):
        result = subprocess.CompletedProcess(['godot'], -6, stdout='partial import diagnostics\n')
        output, errors = io.StringIO(), io.StringIO()
        with patch.object(godot.subprocess, 'run', return_value=result), \
                redirect_stdout(output), redirect_stderr(errors), \
                self.assertRaises(SystemExit) as stopped:
            godot.run(['godot', '--headless', '--import'], capture=True)
        self.assertEqual(stopped.exception.code, 134)
        self.assertIn('SIGABRT', errors.getvalue())
        self.assertIn('partial import diagnostics', output.getvalue())

    def test_logged_engine_error_still_fails_even_with_exit_zero(self):
        result = subprocess.CompletedProcess(['godot'], 0, stdout='SCRIPT ERROR: synthetic failure\n')
        with patch.object(godot.subprocess, 'run', return_value=result), \
                redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as stopped:
            godot.run(['godot'], capture=True)
        self.assertEqual(stopped.exception.code, 1)


if __name__ == '__main__':
    unittest.main()
