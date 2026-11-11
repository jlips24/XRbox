#!/usr/bin/env python3
"""Build a self-contained macOS app. Does not install it or change system settings."""
import argparse
import json
import os
import plistlib
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--debug', action='store_true', help='Package the faster development build')
parser.add_argument('--sign-identity', default='-',
                    help='Existing code-signing identity; default is ad-hoc signing')
args = parser.parse_args()
if sys.platform != 'darwin':
    sys.exit('This packager runs on macOS only.')
cmd = ['cargo', 'build', '--locked', '-p', 'xrbox']
if not args.debug:
    cmd.append('--release')
subprocess.run(cmd, cwd=root, check=True)
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--no-deps', '--format-version=1'], cwd=root))
version = next(p['version'] for p in metadata['packages'] if p['name'] == 'xrbox')
profile = 'debug' if args.debug else 'release'
binary = Path(metadata['target_directory']) / profile / 'xrbox'
# This package deliberately uses native VideoToolbox and statically linked Opus.
# Refuse packages that accidentally require a developer's Homebrew libraries.
for line in subprocess.check_output(['otool', '-L', str(binary)], text=True).splitlines()[1:]:
    library = line.strip().split(' (')[0]
    if not library.startswith(('/System/Library/', '/usr/lib/')):
        sys.exit('Non-system dynamic dependency must be bundled before packaging: ' + library)
app = root / 'dist' / 'XRBox.app'
if app.exists():
    shutil.rmtree(app)
macos = app / 'Contents' / 'MacOS'
resources = app / 'Contents' / 'Resources'
macos.mkdir(parents=True)
resources.mkdir()
shutil.copy2(binary, macos / 'xrbox')
for name in ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
    shutil.copy2(root / name, resources / name)
with (app / 'Contents' / 'Info.plist').open('wb') as stream:
    plistlib.dump({
        'CFBundleName': 'XRBox', 'CFBundleDisplayName': 'XRBox',
        'CFBundleIdentifier': 'org.xrbox.desktop', 'CFBundleExecutable': 'xrbox',
        'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': version,
        'CFBundleVersion': version, 'NSHighResolutionCapable': True,
        'NSBluetoothAlwaysUsageDescription': 'Use a paired game controller for Xbox Remote Play.',
        'NSLocalNetworkUsageDescription': 'Connect directly to your Xbox for console Remote Play.',
    }, stream)
subprocess.run(['codesign', '--force', '--sign', args.sign_identity, str(app)], check=True)
subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
print(app)
print(('Ad-hoc signed' if args.sign_identity == '-' else 'Identity-signed') +
      ' development build; not notarized. No installation or system changes performed.')
