#!/usr/bin/env python3
"""Build/stage XR native libraries or run Godot checks; no tool installation."""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    'aarch64-apple-darwin': ('macos-arm64', 'libxrbox_godot.dylib'),
    'aarch64-unknown-linux-gnu': ('linux-arm64', 'libxrbox_godot.so'),
    'x86_64-unknown-linux-gnu': ('linux-x86_64', 'libxrbox_godot.so'),
    'aarch64-linux-android': ('android-arm64', 'libxrbox_godot.so'),
}


# A normal inferior exit must preserve its status. A debugger stop (including a
# fatal signal) must print the live stack and fail, never inherit GDB's default 0.
GDB_COMMANDS = """set pagination off
set confirm off
set debuginfod enabled off
run
if $_isvoid($_exitcode)
  thread apply all bt full
  info sharedlibrary
  quit 1
else
  quit $_exitcode
end
"""


def run_import_with_backtrace(command):
    """Debug the original cold import; rerunning may no longer reproduce it."""
    directory = ROOT / '.tools'
    directory.mkdir(exist_ok=True)
    path = directory / 'godot-import-backtrace.log'
    print(f'Running original import under GDB: {path}', flush=True)
    with tempfile.TemporaryDirectory(prefix='xrbox-gdb-') as temporary:
        script = Path(temporary) / 'import.gdb'
        script.write_text(GDB_COMMANDS)
        debugger = ['gdb', '--batch', '-q', '-x', str(script), '--args'] + command
        try:
            result = subprocess.run(debugger, cwd=ROOT, text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=180)
        except subprocess.TimeoutExpired as error:
            output = error.stdout or b''
            if isinstance(output, bytes):
                output = output.decode(errors='replace')
            path.write_text(output + '\nDebugger timed out; import failed.\n')
            raise
        path.write_text(result.stdout)
        return result


def run(command, capture=False):
    debug_import = (capture and '--import' in command and sys.platform == 'linux'
                    and os.environ.get('XRBOX_GODOT_CRASH_BACKTRACE') == '1')
    if debug_import:
        result = run_import_with_backtrace(command)
    else:
        result = subprocess.run(command, cwd=ROOT, text=True,
                                stdout=subprocess.PIPE if capture else None,
                                stderr=subprocess.STDOUT if capture else None,
                                timeout=180 if capture else None)
    if capture:
        print(result.stdout, end='')
    if result.returncode < 0:
        name = signal.Signals(-result.returncode).name
        print(f'Command terminated by {name} ({-result.returncode}): {command[0]}', file=sys.stderr)
        raise SystemExit(128 - result.returncode)
    if result.returncode or (capture and any(marker in result.stdout for marker in
                                            ('SCRIPT ERROR:', 'ERROR:'))):
        raise SystemExit(result.returncode or 1)
    return result.stdout


def build_command(cargo, target, host, release=False, workspace_features=False):
    if workspace_features and target != host:
        raise ValueError('Full-workspace feature reuse is for host checks, not Android/cross-target builds')
    command = [cargo, 'build', '--locked']
    if workspace_features:
        command += ['--workspace', '--all-targets', '--all-features']
    else:
        command += ['-p', 'xrbox-godot']
    if target != host:
        command += ['--target', target]
    if release:
        command.append('--release')
    return command


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['build', 'check', 'render', 'run'])
    parser.add_argument('--target', choices=TARGETS)
    parser.add_argument('--workspace-features', action='store_true', help='Host build with the full workspace test feature graph')
    parser.add_argument('--release', action='store_true')
    parser.add_argument('--ui-preview', action='store_true', help='Show synthetic product pages without sign-in')
    parser.add_argument('--desktop', action='store_true', help='Preview without OpenXR')
    args = parser.parse_args()
    cargo = os.environ.get('CARGO', 'cargo')
    host = next(line.split(': ', 1)[1] for line in subprocess.check_output(
        ['rustc', '-vV'], cwd=ROOT, text=True).splitlines() if line.startswith('host: '))
    target = args.target or host
    if target not in TARGETS:
        parser.error('Unsupported native target: ' + target)
    if args.action != 'build' and target != host:
        parser.error('Run/check requires the host library; use build to stage cross-target libraries.')
    if args.action != 'build' and args.release:
        parser.error('Editor run/check uses debug libraries; --release is for exported builds.')
    platform, library = TARGETS[target]
    profile = 'release' if args.release else 'debug'
    if args.workspace_features and args.action != 'build':
        parser.error('--workspace-features applies only to build')
    if args.action == 'build':
        try:
            command = build_command(cargo, target, host, args.release, args.workspace_features)
        except ValueError as error:
            parser.error(str(error))
        run(command)
        metadata = json.loads(subprocess.check_output(
            [cargo, 'metadata', '--locked', '--no-deps', '--format-version=1'], cwd=ROOT))
        build_root = Path(metadata['target_directory'])
        if target != host:
            build_root /= target
        source = build_root / profile / library
        dest = ROOT / 'xr' / 'native' / platform / profile / library
        dest.parent.mkdir(parents=True, exist_ok=True)
        # Replace the inode: rebuilding must not truncate a loaded shared library.
        with tempfile.NamedTemporaryFile(dir=dest.parent, delete=False) as temporary:
            staging = Path(temporary.name)
        try:
            shutil.copy2(source, staging)
            os.replace(staging, dest)
        finally:
            staging.unlink(missing_ok=True)
        print('Staged:', dest)
        return
    binary = os.environ.get('GODOT')
    if not binary:
        local = ROOT / '.tools/godot/Godot.app/Contents/MacOS/Godot'
        binary = str(local) if local.is_file() else shutil.which('godot')
    if not binary:
        parser.error('Godot 4.7.2 required. Set GODOT to its executable path; nothing is installed automatically.')
    version = subprocess.check_output([binary, '--version'], text=True).strip()
    if not version.startswith('4.7.2.stable.'):
        parser.error('Expected pinned Godot 4.7.2 stable; found ' + version)
    if not (ROOT / 'xr' / 'native' / platform / profile / library).is_file():
        parser.error('Host native library missing. Run make xr-build first.')
    common = [binary, '--audio-driver', 'Dummy', '--path', str(ROOT / 'xr')]
    def logged(name, arguments):
        return common + ['--log-file', str(ROOT / '.tools' / name)] + arguments
    (ROOT / '.tools').mkdir(exist_ok=True)
    if args.action == 'check':
        run(logged('godot-import.log', ['--verbose', '--headless', '--xr-mode', 'off', '--import']), capture=True)
        run(logged('godot-check.log', ['--headless', '--xr-mode', 'off', '--script', 'res://tests/smoke.gd', '--', '--test']), capture=True)
    elif args.action == 'render':
        run(logged('godot-render.log', ['--xr-mode', 'off', '--script', 'res://tests/render.gd', '--', '--test', '--ui-preview']), capture=True)
    else:
        command = logged('godot-run.log', ['--xr-mode', 'off', '--', '--desktop'] if args.desktop else [])
        if args.ui_preview:
            if '--' not in command:
                command.append('--')
            command.append('--ui-preview')
        run(command)


if __name__ == '__main__':
    main()
