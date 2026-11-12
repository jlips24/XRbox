#!/usr/bin/env python3
"""Build/verify a development Quest APK. Never installs tools or accesses devices."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import sys
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
APK_NAME = 'XRBox-0.1.10-xr.1-quest-debug.apk'


def digest(path, algorithm='sha256'):
    result = hashlib.new(algorithm)
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            result.update(chunk)
    return result.hexdigest()


def run(command, env, capture=False):
    result = subprocess.run([str(item) for item in command], cwd=ROOT, env=env,
                            text=True, stdout=subprocess.PIPE if capture else None,
                            stderr=subprocess.STDOUT if capture else None)
    if result.returncode:
        raise RuntimeError(result.stdout or ('Build command failed: ' + str(command[0])))
    if capture and any(marker in result.stdout for marker in ('ERROR:', 'SCRIPT ERROR:')):
        raise RuntimeError(result.stdout)
    return result.stdout


def configure_editor_text(text, sdk, java):
    """Only called for the explicitly scoped self-contained editor resource."""
    if '[resource]\n' not in text:
        raise ValueError('Invalid project-local editor settings resource')
    for key, value in {'export/android/android_sdk_path': str(sdk),
                       'export/android/java_sdk_path': str(java),
                       'export/android/shutdown_adb_on_exit': False}.items():
        line = key + ' = ' + json.dumps(value, ensure_ascii=False)
        pattern = r'^' + re.escape(key) + r' = .*?$'
        if re.search(pattern, text, re.M):
            text = re.sub(pattern, lambda _: line, text, flags=re.M)
        else:
            text = text.replace('[resource]\n', '[resource]\n' + line + '\n', 1)
    return text


def validate_elf(data):
    if len(data) < 64 or data[:6] != b'\x7fELF\x02\x01':
        raise ValueError('Expected a little-endian ELF64 shared library')
    if struct.unpack_from('<HH', data, 16) != (3, 183):
        raise ValueError('Expected an AArch64 shared library')
    offset = struct.unpack_from('<Q', data, 32)[0]
    size, count = struct.unpack_from('<HH', data, 54)
    if size < 56 or offset + size * count > len(data):
        raise ValueError('Invalid ELF program headers')
    alignments = [struct.unpack_from('<Q', data, offset + i * size + 48)[0]
                  for i in range(count)
                  if struct.unpack_from('<I', data, offset + i * size)[0] == 1]
    if not alignments or any(a < 16384 or a & (a - 1) for a in alignments):
        raise ValueError('Native load segments must support 16 KiB alignment')
    return alignments


def validate_native(apk):
    required = {'libgodot_android.so', 'libxrbox_godot.so',
                'libgodotopenxrvendors.so', 'libopenxr_loader.so', 'libc++_shared.so'}
    with zipfile.ZipFile(apk) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError('Duplicate APK zip entries')
        libraries = [n for n in names if n.startswith('lib/') and n.endswith('.so')]
        if any(not n.startswith('lib/arm64-v8a/') for n in libraries):
            raise ValueError('Non-ARM64 native library in Quest APK')
        if not required.issubset({Path(n).name for n in libraries}):
            raise ValueError('Missing native core, renderer or OpenXR library')
        if sum(Path(n).name == 'libopenxr_loader.so' for n in libraries) != 1:
            raise ValueError('Expected exactly one OpenXR loader')
        if any(n.endswith(('.keystore', '.jks', '.dylib', '.dll', '.exe')) or n.startswith('assets/tests/') for n in names):
            raise ValueError('Signing keys or test scripts must not be packaged')
        return {n: validate_elf(archive.read(n)) for n in libraries}


def stage_vendor(pin):
    addon = ROOT / 'xr/addons/godotopenxrvendors'
    marker = addon / '.xrbox-source-sha256'
    if addon.exists():
        if marker.is_file() and marker.read_text().strip() == pin['sha256']:
            return
        raise RuntimeError('Unmanaged OpenXR addon exists; preserve/review it before staging the pinned dependency')
    cache = ROOT / '.tools/android/downloads/openxr-vendors.zip'
    cache.parent.mkdir(parents=True, exist_ok=True)
    if not cache.is_file():
        temporary = cache.with_suffix('.part')
        with urllib.request.urlopen(pin['url'], timeout=60) as response, temporary.open('wb') as out:
            shutil.copyfileobj(response, out)
        temporary.replace(cache)
    if digest(cache) != pin['sha256']:
        raise ValueError('OpenXR addon archive checksum mismatch')
    prefix = 'asset/addons/godotopenxrvendors/'
    with zipfile.ZipFile(cache) as archive:
        for item in archive.infolist():
            if not item.filename.startswith(prefix) or item.is_dir():
                continue
            relative = Path(item.filename[len(prefix):])
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError('Invalid addon archive path')
            target = addon / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            if stat.S_ISLNK(item.external_attr >> 16):
                link = archive.read(item).decode()
                if not (target.parent / link).resolve().is_relative_to(addon.resolve()):
                    raise ValueError('Invalid addon symlink')
                target.symlink_to(link)
            else:
                target.write_bytes(archive.read(item))
                mode = (item.external_attr >> 16) & 0o777
                if mode:
                    target.chmod(mode)
    marker.write_text(pin['sha256'] + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    args = parser.parse_args()
    pins = json.loads((ROOT / 'xr/toolchain.json').read_text())
    sdk = Path(os.environ.get('XRBOX_ANDROID_SDK', ROOT / '.tools/android/sdk')).resolve()
    java_value = os.environ.get('JAVA_HOME')
    if not java_value and sys.platform == 'darwin':
        java_value = subprocess.check_output(['/usr/libexec/java_home', '-v', '17'], text=True).strip()
    if not java_value:
        parser.error('Set a command-local JAVA_HOME for the required JDK 17')
    java = Path(java_value).resolve()
    ndk = sdk / 'ndk' / pins['android_sdk_ndk']['ndk']
    host = {'darwin': 'darwin-x86_64', 'linux': 'linux-x86_64'}.get(sys.platform)
    if not host:
        parser.error('This helper currently supports macOS and Linux hosts')
    toolbin = ndk / 'toolchains/llvm/prebuilt' / host / 'bin'
    tools = sdk / 'build-tools' / pins['android_sdk_ndk']['build_tools']
    env = dict(os.environ, JAVA_HOME=str(java), ANDROID_HOME=str(sdk), ANDROID_SDK_ROOT=str(sdk),
               GRADLE_USER_HOME=str(ROOT / '.tools/android/gradle'),
               CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=str(toolbin / ('aarch64-linux-android' + str(pins['android_sdk_ndk']['rust_link_api']) + '-clang')))
    apk = ROOT / 'dist/xr/android' / APK_NAME
    for path in [java / 'bin/java', tools / 'apksigner', tools / 'aapt2', tools / 'zipalign',
                 sdk / ('platforms/android-' + str(pins['android_sdk_ndk']['compile_sdk'])) / 'android.jar', toolbin / 'llvm-strip']:
        if not path.is_file():
            parser.error('Missing prerequisite (see xr/ANDROID.md): ' + str(path))
    if not args.verify_only:
        godot = Path(os.environ.get('GODOT', ROOT / '.tools/godot/Godot.app/Contents/MacOS/Godot')).resolve()
        if not godot.is_relative_to((ROOT / '.tools').resolve()):
            parser.error('Use a project-local self-contained Godot; global editor settings are not changed')
        if not any((godot.parent / marker).is_file() for marker in ('_sc_', '._sc_')):
            parser.error('Enable self-contained mode beside the project-local Godot binary first')
        version = run([godot, '--version'], env, capture=True).strip()
        if not version.startswith('4.7.2.stable.'):
            parser.error('Pinned Godot 4.7.2 stable is required')
        editor = ROOT / '.tools/godot/editor_data/editor_settings-4.7.tres'
        if not editor.is_file():
            parser.error('Initialize the self-contained editor first (see xr/ANDROID.md)')
        stage_vendor(pins['openxr_vendors_plugin'])
        updated = configure_editor_text(editor.read_text(), sdk, java)
        temporary = editor.with_suffix('.tmp')
        temporary.write_text(updated)
        temporary.replace(editor)
        key = ROOT / '.tools/android/debug.keystore'
        if not key.exists():
            run([java / 'bin/keytool', '-genkeypair', '-keystore', key, '-storepass', 'android',
                 '-keypass', 'android', '-alias', 'androiddebugkey', '-dname',
                 'CN=Android Debug,O=XRBox,C=US', '-validity', '10000', '-keyalg', 'RSA', '-keysize', '2048'], env)
        env.update(GODOT_ANDROID_KEYSTORE_DEBUG_PATH=str(key), GODOT_ANDROID_KEYSTORE_DEBUG_USER='androiddebugkey',
                   GODOT_ANDROID_KEYSTORE_DEBUG_PASSWORD='android')
        # The editor must also be able to load the host GDExtension during export.
        run([sys.executable, ROOT / 'scripts/godot.py', 'build'], env)
        run([sys.executable, ROOT / 'scripts/godot.py', 'build', '--target', 'aarch64-linux-android'], env)
        run([toolbin / 'llvm-strip', '--strip-debug', ROOT / 'xr/native/android-arm64/debug/libxrbox_godot.so'], env)
        legal = ROOT / 'xr/legal'
        legal.mkdir(exist_ok=True)
        for filename in ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
            shutil.copy2(ROOT / filename, legal / (filename + '.txt'))
        for filename, source in pins['android_notices'].items():
            cache = ROOT / '.tools/android/downloads' / filename
            if not cache.is_file():
                with urllib.request.urlopen(source['url'], timeout=30) as response, cache.open('wb') as output:
                    shutil.copyfileobj(response, output)
            if digest(cache) != source['sha256']:
                raise ValueError('Notice checksum mismatch: ' + filename)
            shutil.copy2(cache, legal / filename)
        for source in (ROOT / 'xr/addons/godotopenxrvendors').rglob('LICENSE*'):
            shutil.copy2(source, legal / (source.parent.name + '-' + source.name + '.txt'))
        apk.parent.mkdir(parents=True, exist_ok=True)
        command = [godot, '--headless', '--xr-mode', 'off', '--audio-driver', 'Dummy', '--path', ROOT / 'xr',
                   '--log-file', ROOT / '.tools/android/godot-export.log']
        if not (ROOT / 'xr/android/build/.build_version').is_file():
            command.append('--install-android-build-template')
        run(command + ['--export-debug', 'Meta Quest Debug', apk], env, capture=True)
    libraries = validate_native(apk)
    run([tools / 'apksigner', 'verify', '--verbose', apk], env)
    run([tools / 'zipalign', '-c', '-P', '16', '4', apk], env)
    manifest = run([tools / 'aapt2', 'dump', 'xmltree', apk, '--file', 'AndroidManifest.xml'], env, capture=True)
    for required in ['org.xrbox.prototype', 'quest3|quest3s', 'com.oculus.intent.category.VR',
                     'org.khronos.openxr.intent.category.IMMERSIVE_HMD', '0.1.10-xr.1']:
        if required not in manifest:
            raise ValueError('Missing APK manifest entry: ' + required)
    if any(permission in manifest for permission in ['android.permission.CAMERA', 'android.permission.RECORD_AUDIO',
                                                     'android.permission.INTERNET']):
        raise ValueError('Prototype requests an unnecessary camera/microphone/network permission')
    checksum = digest(apk)
    apk.with_suffix('.apk.sha256').write_text(checksum + '  ' + apk.name + '\n')
    print('Verified development APK:', apk)
    print('Bytes:', apk.stat().st_size, 'SHA256:', checksum)
    print('ARM64 libraries with 16 KiB load alignment:', len(libraries))
    print('Install and XR/controller testing on Quest 3S remain manual. No streaming is implemented here.')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from None
