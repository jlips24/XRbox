#!/usr/bin/env python3
"""Build/stage XR native libraries or run Godot checks; no tool installation."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    'aarch64-apple-darwin': ('macos-arm64', 'libxrbox_godot.dylib'),
    'aarch64-unknown-linux-gnu': ('linux-arm64', 'libxrbox_godot.so'),
    'x86_64-unknown-linux-gnu': ('linux-x86_64', 'libxrbox_godot.so'),
    'aarch64-linux-android': ('android-arm64', 'libxrbox_godot.so'),
}


def run(command, capture=False):
    result = subprocess.run(command, cwd=ROOT, text=True,
                            stdout=subprocess.PIPE if capture else None,
                            stderr=subprocess.STDOUT if capture else None,
                            timeout=180 if capture else None)
    if capture:
        print(result.stdout, end='')
    if result.returncode or (capture and any(marker in result.stdout for marker in
                                            ('SCRIPT ERROR:', 'ERROR:'))):
        raise SystemExit(result.returncode or 1)
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['build', 'check', 'render', 'run'])
    parser.add_argument('--target', choices=TARGETS)
    parser.add_argument('--release', action='store_true')
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
    if args.action == 'build':
        command = [cargo, 'build', '--locked', '-p', 'xrbox-godot']
        if target != host:
            command += ['--target', target]
        if args.release:
            command.append('--release')
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
    common = [binary, '--audio-driver', 'Dummy', '--path', str(ROOT / 'xr'), '--log-file', str(ROOT / '.tools/godot-run.log')]
    (ROOT / '.tools').mkdir(exist_ok=True)
    if args.action == 'check':
        run(common + ['--headless', '--xr-mode', 'off', '--import'], capture=True)
        run(common + ['--headless', '--xr-mode', 'off', '--script', 'res://tests/smoke.gd', '--', '--test'], capture=True)
    elif args.action == 'render':
        run(common + ['--xr-mode', 'off', '--script', 'res://tests/render.gd', '--', '--test'], capture=True)
    else:
        command = common + (['--xr-mode', 'off', '--', '--desktop'] if args.desktop else [])
        run(command)


if __name__ == '__main__':
    main()
