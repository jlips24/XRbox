#!/usr/bin/env python3
"""Build a Linux ARM64 Flatpak; never installs host tools, runtimes or the app."""
import argparse
import ctypes
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import zipfile

import ci
import release_version

ROOT = Path(__file__).resolve().parents[1]
APP_ID = 'io.github.jlips24.XRbox'


def bundle_name(version):
    return f'XRbox-{version}-linux-arm64.flatpak'


def run(command, **kwargs):
    subprocess.run([str(part) for part in command], cwd=ROOT, check=True, **kwargs)


def copy_sources(destination):
    """Copy versioned/nonignored source only; never local keys, logs or outputs."""
    names = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others',
                                     '--exclude-standard'], cwd=ROOT).decode().split('\0')
    for name in sorted(set(names) - {''}):
        source = ROOT / name
        if source.is_symlink():
            raise ValueError('Source staging does not accept symlinks: ' + name)
        if source.is_file():
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)


def check_output(text):
    if 'ERROR:' in text or 'SCRIPT ERROR:' in text:
        raise ValueError('Godot export/smoke reported an error; see captured output')


def validate_export(directory):
    """Reject extension-only output and the wrong engine/native architecture."""
    engine, pck = directory / 'xrbox.arm64', directory / 'xrbox.pck'
    libraries = list(directory.rglob('libxrbox_godot.so'))
    if not pck.is_file() or pck.stat().st_size == 0 or not libraries:
        raise ValueError('Incomplete Linux export: executable, PCK and GDExtension are required')
    for path in [engine, *libraries]:
        with path.open('rb') as stream:
            header = stream.read(20)
        if (len(header) != 20 or header[:6] != b'\x7fELF\x02\x01'
                or int.from_bytes(header[18:20], 'little') != 183):
            raise ValueError('Linux export must contain ARM64 ELF binaries: ' + str(path))


def check_sdk_libclang():
    """Check the build-sandbox library, never a host compiler or runtime dependency."""
    directory = os.environ.get('LIBCLANG_PATH')
    if not directory:
        raise ValueError('Flatpak SDK requires LIBCLANG_PATH from the LLVM SDK extension; see docs/linux.md')
    candidates = sorted(set(Path(directory).glob('libclang.so*')) |
                        set(Path(directory).glob('libclang-*.so*')))
    for candidate in candidates:
        try:
            library = ctypes.CDLL(str(candidate))
            getattr(library, 'clang_getClangVersion')
            return
        except (OSError, AttributeError):
            continue
    raise ValueError('Cannot load libclang from ' + directory +
                     '; install/mount org.freedesktop.Sdk.Extension.llvm20 for the pinned SDK branch; see docs/linux.md')


def inside_sdk():
    """Compile against the actual runtime SDK, then export the complete Godot app."""
    check_sdk_libclang()
    config = release_version.sync(ROOT)
    cargo = os.environ.get('CARGO', 'cargo')
    rustc = os.environ.get('RUSTC', 'rustc')
    pinned = json.loads((ROOT / 'xr/toolchain.json').read_text())
    version = subprocess.check_output([rustc, '--version'], text=True).split()[1]
    if version != pinned['rust']:
        raise ValueError('Flatpak compiler must match the project-pinned Rust toolchain')
    run([cargo, 'build', '--frozen', '--release', '-p', 'xrbox-godot'])
    metadata = json.loads(subprocess.check_output(
        [cargo, 'metadata', '--offline', '--locked', '--no-deps', '--format-version=1'], cwd=ROOT))
    library = Path(metadata['target_directory']) / 'release/libxrbox_godot.so'
    for profile in ['debug', 'release']:
        target = ROOT / 'xr/native/linux-arm64' / profile
        target.mkdir(parents=True, exist_ok=True)
        shutil.copy2(library, target / library.name)
    legal = ROOT / 'xr/legal'
    legal.mkdir(exist_ok=True)
    for source in [ROOT / 'LICENSE', ROOT / 'THIRD_PARTY_NOTICES.md', ROOT / 'xr/fonts/Hack-Regular.txt']:
        shutil.copy2(source, legal / (source.name + '.txt'))
    for source in (ROOT / 'xr/licenses').glob('*.txt'):
        shutil.copy2(source, legal / source.name)
    godot = ROOT / '.tools/godot/godot'
    base = [godot, '--headless', '--audio-driver', 'Dummy', '--xr-mode', 'off', '--path', ROOT / 'xr']
    Path('/app/lib/xrbox').mkdir(parents=True, exist_ok=True)
    for args in [['--editor', '--import', '--quit', '--frame-delay', '1000'],
                 ['--export-release', 'Linux ARM64', '/app/lib/xrbox/xrbox.arm64']]:
        result = subprocess.run([str(p) for p in base + args], cwd=ROOT, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(result.stdout, flush=True)
        result.check_returncode()
        check_output(result.stdout)
    for source, target in [
            ('packaging/flatpak/xrbox', '/app/bin/xrbox'),
            (f'packaging/flatpak/{APP_ID}.desktop', f'/app/share/applications/{APP_ID}.desktop'),
            (f'packaging/flatpak/{APP_ID}.metainfo.xml', f'/app/share/metainfo/{APP_ID}.metainfo.xml'),
            ('xr/icon.svg', f'/app/share/icons/hicolor/scalable/apps/{APP_ID}.svg')]:
        run(['install', '-D', '-m', '755' if target == '/app/bin/xrbox' else '644', ROOT / source, target])
    notices = Path('/app/share/licenses') / APP_ID
    shutil.copytree(legal, notices, dirs_exist_ok=True)
    for name in ['LICENSE.txt', 'COPYRIGHT.txt']:
        shutil.copy2(ROOT / '.tools/godot' / name, notices / ('Godot-' + name))
    (notices / 'FFmpeg-COPYING.LGPLv2.1').write_bytes((ROOT / 'packaging/flatpak/FFmpeg-COPYING.LGPLv2.1').read_bytes())
    print('Exported complete Linux XR prototype', config['version'], flush=True)


def build():
    if platform.system() != 'Linux' or platform.machine() not in ('aarch64', 'arm64'):
        raise ValueError('Build on Linux ARM64 with Flatpak/SDK prerequisites; see docs/linux.md')
    for tool in ['flatpak', 'flatpak-builder', 'cargo', 'rustc', 'eu-strip', 'eu-elfcompress']:
        if not shutil.which(tool):
            raise ValueError('Missing prerequisite: ' + tool + ' (see docs/linux.md)')
    pins = json.loads((ROOT / 'xr/toolchain.json').read_text())
    work = ROOT / '.tools/flatpak'
    work.mkdir(parents=True, exist_ok=True)
    # Only replace our generated input tree. Builder caches and repository persist.
    inputs = work / 'inputs'
    if inputs.exists():
        shutil.rmtree(inputs)
    source = inputs / 'source'
    source.mkdir(parents=True)
    copy_sources(source)
    config = release_version.sync(source)
    vendor = source / 'cargo-vendor'
    cargo_config = subprocess.check_output(['cargo', 'vendor', '--locked', str(vendor)], cwd=ROOT, text=True)
    # cargo vendor prints an absolute directory; the build sandbox has its own path.
    cargo_config = cargo_config.replace(str(vendor), 'cargo-vendor')
    (source / '.cargo').mkdir(exist_ok=True)
    (source / '.cargo/config.toml').write_text(cargo_config)
    sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], cwd=ROOT, text=True).strip())
    for name in ['bin', 'lib']:
        shutil.copytree(sysroot / name, inputs / 'rust-toolchain' / name)
    godot_home = source / '.tools/godot'
    godot_home.mkdir(parents=True)
    editor = ci.download(pins['linux_arm64_archive'], work / 'downloads/godot.zip')
    with zipfile.ZipFile(editor) as archive:
        (godot_home / 'godot').write_bytes(archive.read(pins['linux_arm64_archive']['binary']))
    (godot_home / 'godot').chmod(0o755)
    (godot_home / '_sc_').touch()
    templates = godot_home / 'editor_data/export_templates' / pins['export_templates']['version']
    templates.mkdir(parents=True)
    archive_path = ci.download(pins['export_templates'], work / 'downloads/templates.tpz')
    with zipfile.ZipFile(archive_path) as archive:
        (templates / 'linux_release.arm64').write_bytes(archive.read('templates/linux_release.arm64'))
    for name in ['LICENSE.txt', 'COPYRIGHT.txt']:
        pin = pins['android_notices']['Godot-' + name]
        shutil.copy2(ci.download(pin, work / 'downloads' / name, 'sha256'), godot_home / name)
    ffmpeg = ci.download(pins['flatpak']['ffmpeg'], work / 'downloads/ffmpeg.tar.xz', 'sha256')
    shutil.copy2(ffmpeg, inputs / 'ffmpeg.tar.xz')
    with tarfile.open(ffmpeg) as archive:
        member = next(m for m in archive.getmembers() if m.name.endswith('/COPYING.LGPLv2.1'))
        with archive.extractfile(member) as data:
            (source / 'packaging/flatpak/FFmpeg-COPYING.LGPLv2.1').write_bytes(data.read())
    manifest = json.loads((ROOT / 'packaging/flatpak' / (APP_ID + '.json')).read_text())
    if manifest['runtime-version'] != pins['flatpak']['runtime_version']:
        raise ValueError('Flatpak runtime version differs from toolchain pin')
    path = inputs / (APP_ID + '.json')
    path.write_text(json.dumps(manifest, indent=2) + '\n')
    build_dir, repo = work / 'build', work / 'repo'
    run(['flatpak-builder', '--user', '--arch=aarch64', '--force-clean', '--disable-rofiles-fuse',
         '--state-dir=' + str(work / 'state'), '--repo=' + str(repo), build_dir, path])
    validate_export(build_dir / 'files/lib/xrbox')
    # Inspect under the runtime, not host Ubuntu: missing transitive libraries fail here.
    validation = """set -eu
    test -s /app/lib/xrbox/xrbox.arm64
    test -s /app/lib/xrbox/xrbox.pck
    find /app -name '*.so*' -type f -exec ldd '{}' ';' > /tmp/xrbox-ldd.txt
    ldd /app/lib/xrbox/xrbox.arm64 >> /tmp/xrbox-ldd.txt
    cat /tmp/xrbox-ldd.txt
    ! grep -q 'not found' /tmp/xrbox-ldd.txt
    """
    run(['flatpak', 'build', '--runtime', '--nofilesystem=host', build_dir, 'sh', '-c', validation])
    smoke = subprocess.run(['flatpak', 'build', '--runtime', '--nofilesystem=host', str(build_dir), '/app/bin/xrbox',
                            '--headless', '--audio-driver', 'Dummy', '--xr-mode', 'off', '--quit-after', '10',
                            '--', '--test'], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           timeout=120)
    print(smoke.stdout, flush=True)
    smoke.check_returncode()
    check_output(smoke.stdout)
    output = ROOT / 'dist/xr/linux-arm64' / bundle_name(config['version'])
    output.parent.mkdir(parents=True, exist_ok=True)
    output.unlink(missing_ok=True)
    run(['flatpak', 'build-bundle', '--arch=aarch64',
         '--runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo', repo, output, APP_ID, 'stable'])
    print('Built Linux ARM64 Flatpak:', output)
    print('Synthetic prototype only; Frame/OpenXR/video/audio/controller acceptance remains manual.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inside-sdk', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.inside_sdk:
        inside_sdk()
    else:
        build()


if __name__ == '__main__':
    main()
