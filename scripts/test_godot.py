"""Build-graph and subprocess-failure regressions; no engine or compiler launched."""
from contextlib import redirect_stderr, redirect_stdout
import io
import subprocess
import shutil
import sys
import tempfile
from pathlib import Path
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
        with patch.dict(godot.os.environ, {'XRBOX_GODOT_CRASH_BACKTRACE': '0'}), \
                patch.object(godot.subprocess, 'run', return_value=result), \
                redirect_stdout(output), redirect_stderr(errors), \
                self.assertRaises(SystemExit) as stopped:
            godot.run(['godot', '--headless', '--import'], capture=True)
        self.assertEqual(stopped.exception.code, 134)
        self.assertIn('SIGABRT', errors.getvalue())
        self.assertIn('partial import diagnostics', output.getvalue())

    def test_original_import_runs_once_under_debugger_and_preserves_status(self):
        for status in (0, 1, 7):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                command = ['godot', '--log-file', str(root / 'godot-import.log'), '--headless', '--import']
                def debugger(arguments, **kwargs):
                    self.assertEqual(arguments[0], 'gdb')
                    self.assertEqual(arguments[arguments.index('--args') + 1:], command)
                    script = Path(arguments[arguments.index('-x') + 1]).read_text()
                    self.assertIn('thread apply all bt full', script)
                    self.assertIn('info sharedlibrary', script)
                    self.assertIn('quit $_exitcode', script)
                    self.assertIn('quit 1', script)
                    return subprocess.CompletedProcess(arguments, status, stdout='debugger evidence\n')
                with patch.object(godot, 'ROOT', root), \
                        patch.object(godot.sys, 'platform', 'linux'), \
                        patch.dict(godot.os.environ, {'XRBOX_GODOT_CRASH_BACKTRACE': '1'}), \
                        patch.object(godot.subprocess, 'run', side_effect=debugger) as run, \
                        redirect_stdout(io.StringIO()):
                    if status:
                        with self.assertRaises(SystemExit) as stopped:
                            godot.run(command, capture=True)
                        self.assertEqual(stopped.exception.code, status)
                    else:
                        godot.run(command, capture=True)
                run.assert_called_once()
                self.assertEqual((root / '.tools/godot-import-backtrace.log').read_text(), 'debugger evidence\n')

    def test_debugger_timeout_retains_partial_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(godot, 'ROOT', root), \
                    patch.object(godot.subprocess, 'run', side_effect=subprocess.TimeoutExpired(['gdb'], 180, output=b'partial stack')), \
                    redirect_stdout(io.StringIO()), self.assertRaises(subprocess.TimeoutExpired):
                godot.run_import_with_backtrace(['godot', '--import'])
            self.assertIn('partial stack', (root / '.tools/godot-import-backtrace.log').read_text())

    @unittest.skipUnless(sys.platform == 'linux' and shutil.which('gdb'), 'Linux GDB integration check')
    def test_real_debugger_preserves_exit_and_captures_abort(self):
        cases = [('import sys; sys.exit(0)', 0),
                 ('import sys; sys.exit(7)', 7),
                 ('import os; os.abort()', 1)]
        with tempfile.TemporaryDirectory() as directory, \
                patch.object(godot, 'ROOT', Path(directory)), redirect_stdout(io.StringIO()):
            for program, expected in cases:
                with self.subTest(program=program):
                    result = godot.run_import_with_backtrace([sys.executable, '-c', program])
                    self.assertEqual(result.returncode, expected, result.stdout)
                    if 'abort' in program:
                        self.assertIn('SIGABRT', result.stdout)
                        self.assertIn('#0', result.stdout)
                        self.assertIn('Shared Object Library', result.stdout)

    def test_logged_engine_error_still_fails_even_with_exit_zero(self):
        result = subprocess.CompletedProcess(['godot'], 0, stdout='SCRIPT ERROR: synthetic failure\n')
        with patch.object(godot.subprocess, 'run', return_value=result), \
                redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as stopped:
            godot.run(['godot'], capture=True)
        self.assertEqual(stopped.exception.code, 1)


if __name__ == '__main__':
    unittest.main()
