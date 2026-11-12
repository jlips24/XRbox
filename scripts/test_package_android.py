"""Packaging regressions: reject wrong ABI, malformed ELF and unsafe dependencies."""
import io
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import package_android as packaging


def elf(machine=183, alignment=16384):
    data = bytearray(120)
    data[:6] = b'\x7fELF\x02\x01'
    struct.pack_into('<HH', data, 16, 3, machine)
    struct.pack_into('<Q', data, 32, 64)
    struct.pack_into('<HH', data, 54, 56, 1)
    struct.pack_into('<I', data, 64, 1)
    struct.pack_into('<Q', data, 112, alignment)
    return data


class PackagingTests(unittest.TestCase):
    def test_editor_paths_replace_only_selected_local_fields_and_escape_paths(self):
        text = '[gd_resource type="EditorSettings" format=3]\n[resource]\nkeep = true\nexport/android/java_sdk_path = "old"\n'
        updated = packaging.configure_editor_text(text, Path('/sdk/with "quotes"'), Path('/jdk/17'))
        self.assertIn('keep = true', updated)
        self.assertNotIn('"old"', updated)
        self.assertIn('export/android/java_sdk_path = "/jdk/17"', updated)
        self.assertIn(r'/sdk/with \"quotes\"', updated)
        self.assertIn('export/android/shutdown_adb_on_exit = false', updated)
        with self.assertRaises(ValueError):
            packaging.configure_editor_text('invalid', Path('/sdk'), Path('/jdk'))

    def test_elf_checks_architecture_alignment_and_program_bounds(self):
        self.assertEqual(packaging.validate_elf(elf()), [16384])
        wrong_class = elf()
        wrong_class[4] = 1
        invalid_offset = elf()
        struct.pack_into('<Q', invalid_offset, 32, 4096)
        for data in [b'not ELF', elf(machine=62), elf(alignment=4096), elf(alignment=20000),
                     wrong_class, invalid_offset]:
            with self.subTest(header=bytes(data[:20])):
                with self.assertRaises(ValueError):
                    packaging.validate_elf(data)

    def test_apk_requires_arm64_core_and_one_loader_and_excludes_keys(self):
        names = ['libgodot_android.so', 'libxrbox_godot.so', 'libgodotopenxrvendors.so',
                 'libopenxr_loader.so', 'libc++_shared.so']

        def apk(extra=None, omit=None):
            result = io.BytesIO()
            with zipfile.ZipFile(result, 'w') as archive:
                for name in names:
                    if name != omit:
                        archive.writestr('lib/arm64-v8a/' + name, elf())
                if extra:
                    archive.writestr(extra, elf())
            result.seek(0)
            return result

        self.assertEqual(len(packaging.validate_native(apk())), 5)
        for extra in ['lib/x86_64/libxrbox_godot.so', 'assets/debug.keystore', 'assets/tests/smoke.gdc', 'assets/native/libxrbox.dylib']:
            with self.subTest(extra=extra):
                with self.assertRaises(ValueError):
                    packaging.validate_native(apk(extra=extra))
        with self.assertRaises(ValueError):
            packaging.validate_native(apk(omit='libxrbox_godot.so'))

    def test_tampered_or_escaping_vendor_archives_cannot_be_staged(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(packaging, 'ROOT', Path(temporary)):
            cache = Path(temporary) / '.tools/android/downloads/openxr-vendors.zip'
            cache.parent.mkdir(parents=True)
            with zipfile.ZipFile(cache, 'w') as archive:
                archive.writestr('asset/addons/godotopenxrvendors/../escape.txt', 'bad')
            pin = {'url': 'https://unused.invalid', 'sha256': 'wrong'}
            with self.assertRaisesRegex(ValueError, 'checksum'):
                packaging.stage_vendor(pin)
            pin['sha256'] = packaging.digest(cache)
            with self.assertRaisesRegex(ValueError, 'path'):
                packaging.stage_vendor(pin)
            self.assertFalse((Path(temporary) / 'xr/addons/escape.txt').exists())


if __name__ == '__main__':
    unittest.main()
