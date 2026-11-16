#!/usr/bin/env python3
"""GitHub-runner setup and release-name gating; never changes host defaults."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TAG = re.compile(r'v[0-9]+\.[0-9]+(?:\.[0-9]+)?', re.ASCII)


def valid_tag(value):
    return TAG.fullmatch(value) is not None


def digest(path, algorithm):
    value = hashlib.new(algorithm)
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def download(pin, destination, algorithm='sha512'):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.is_file() or digest(destination, algorithm) != pin[algorithm]:
        temporary = destination.with_suffix('.download')
        try:
            with urllib.request.urlopen(pin['url'], timeout=60) as source, temporary.open('wb') as output:
                shutil.copyfileobj(source, output)
            if digest(temporary, algorithm) != pin[algorithm]:
                raise ValueError('Download checksum mismatch: ' + destination.name)
            temporary.replace(destination)
        finally:
            temporary.unlink(missing_ok=True)
    return destination


def environment(name, value):
    # Only called on ephemeral GitHub runners, never shell startup/global config.
    with Path(os.environ['GITHUB_ENV']).open('a') as output:
        output.write(f'{name}={value}\n')


def setup_godot():
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        raise ValueError('Godot CI setup is qualified for Linux x86_64 runners')
    pins = json.loads((ROOT / 'xr/toolchain.json').read_text())
    pin = pins['linux_x86_64_archive']
    archive = download(pin, ROOT / '.tools/ci/godot-linux.zip')
    home = ROOT / '.tools/godot'
    home.mkdir(parents=True, exist_ok=True)
    binary = home / 'godot'
    with zipfile.ZipFile(archive) as source:
        binary.write_bytes(source.read(pin['binary']))
    binary.chmod(0o755)
    (home / '_sc_').touch()
    data = home / 'editor_data'
    data.mkdir(exist_ok=True)
    settings = data / 'editor_settings-4.7.tres'
    if not settings.exists():
        settings.write_text('[gd_resource type="EditorSettings" format=3]\n\n[resource]\n')
    found = subprocess.check_output([binary, '--version'], text=True).strip()
    if not found.startswith(pins['godot'] + '.'):
        raise ValueError('Downloaded Godot does not match the pinned version')
    environment('GODOT', binary)


def setup_android():
    pins = json.loads((ROOT / 'xr/toolchain.json').read_text())
    sdk_pins = pins['android_sdk_ndk']
    java = Path(os.environ['JAVA_HOME_17_X64'])
    if not (java / 'bin/java').is_file():
        raise ValueError('GitHub runner must provide JDK 17')
    # Use the runner's existing SDK manager to install only into our local SDK.
    bootstrap = Path(os.environ['ANDROID_HOME']) / 'cmdline-tools/latest/bin/sdkmanager'
    if not bootstrap.is_file():
        raise ValueError('GitHub runner must provide an Android SDK manager')
    sdk = ROOT / '.tools/android/sdk'
    sdk.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, JAVA_HOME=str(java))
    base = [str(bootstrap), '--sdk_root=' + str(sdk)]
    subprocess.run(base + ['--licenses'], env=env, input='y\n' * 100, text=True, check=True)
    subprocess.run(base + ['cmdline-tools;' + sdk_pins['command_line_tools'],
                          'platform-tools', 'build-tools;' + sdk_pins['build_tools'],
                          'platforms;android-' + str(sdk_pins['compile_sdk']),
                          'ndk;' + sdk_pins['ndk']],
                   env=env, input='y\n' * 100, text=True, check=True)
    archive = download(pins['export_templates'], ROOT / '.tools/ci/templates.tpz')
    templates = ROOT / '.tools/godot/editor_data/export_templates' / pins['export_templates']['version']
    templates.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive) as source:
        for name in ['android_debug.apk', 'android_release.apk', 'android_source.zip']:
            (templates / name).write_bytes(source.read('templates/' + name))
    environment('JAVA_HOME', java)
    environment('XRBOX_ANDROID_SDK', sdk)


def validate_release_version(tag, app_version):
    if not valid_tag(tag) or tag[1:] != app_version:
        raise ValueError('Release tag must match the Godot APK version (without the v prefix)')


def metadata(directory):
    directory.mkdir(parents=True, exist_ok=True)
    core = re.search(r'^version\s*=\s*"([^"]+)"', (ROOT / 'crates/xrbox-core/Cargo.toml').read_text(), re.M).group(1)
    app = re.search(r'^config/version="([^"]+)"', (ROOT / 'xr/project.godot').read_text(), re.M).group(1)
    validate_release_version(os.environ['XRBOX_TAG'], app)
    target = {'linux-arm64-extension': 'aarch64-unknown-linux-gnu',
              'android-development-apk': 'aarch64-linux-android'}[os.environ['XRBOX_BUILD_KIND']]
    info = {
        'tag': os.environ['XRBOX_TAG'],
        'kind': os.environ['XRBOX_BUILD_KIND'],
        'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'core_version': core, 'app_version': app,
        'architecture': 'aarch64', 'target': target,
        'build_host_architecture': platform.machine(),
        'hardware_validated': False,
        'sha256': {str(path.relative_to(directory)): digest(path, 'sha256')
                   for path in sorted(directory.rglob('*')) if path.is_file() and path.name != 'build.json'},
    }
    (directory / 'build.json').write_text(json.dumps(info, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['rust-version', 'tag', 'godot', 'android', 'metadata'])
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    action = args.action
    if action == 'rust-version':
        print(re.search(r'^channel\s*=\s*"([^"]+)"', (ROOT / 'rust-toolchain.toml').read_text(), re.M).group(1))
        return
    if os.environ.get('GITHUB_ACTIONS') != 'true':
        parser.error('Setup/tag actions run only on ephemeral GitHub Actions runners')
    if action == 'tag':
        tag = os.environ['XRBOX_TAG']
        with Path(os.environ['GITHUB_OUTPUT']).open('a') as output:
            output.write('eligible=' + str(valid_tag(tag)).lower() + '\n')
            if valid_tag(tag):
                output.write('tag=' + tag + '\n')
        print('Release name accepted' if valid_tag(tag) else 'Skipped: name must match vX.X or vX.X.X')
    elif action == 'metadata':
        if args.output is None or not valid_tag(os.environ['XRBOX_TAG']):
            parser.error('metadata requires --output and a valid XRBOX_TAG')
        metadata(args.output)
    elif action == 'godot':
        setup_godot()
    else:
        setup_android()


if __name__ == '__main__':
    main()
