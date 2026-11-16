"""Pure APK/native inspection; no SDK, editor or device access."""
from pathlib import Path
import struct
import zipfile


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


