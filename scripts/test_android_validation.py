"""Native APK inspection regressions use synthetic ELF and ZIP bytes only."""
import io
import struct
import unittest
import zipfile

import android_validation as validation


def elf(machine=183, alignment=16384):
    data = bytearray(120)
    data[:6] = b'\x7fELF\x02\x01'
    struct.pack_into('<HH', data, 16, 3, machine)
    struct.pack_into('<Q', data, 32, 64)
    struct.pack_into('<HH', data, 54, 56, 1)
    struct.pack_into('<I', data, 64, 1)
    struct.pack_into('<Q', data, 112, alignment)
    return data


class AndroidValidationTests(unittest.TestCase):
    def test_manifest_matches_configured_version_and_rejects_old_versions(self):
        manifest = ('org.xrbox.prototype android.permission.INTERNET quest3|quest3s '
                    'com.oculus.intent.category.VR org.khronos.openxr.intent.category.IMMERSIVE_HMD\n'
                    'A: android:versionName(0x0101021c)="0.0.1"')
        validation.validate_manifest(manifest, '0.0.1')
        with self.assertRaisesRegex(ValueError, 'versionName'):
            validation.validate_manifest(manifest, '0.1.10-xr.3')
        with self.assertRaisesRegex(ValueError, 'versionName'):
            validation.validate_manifest(manifest.replace('versionName', 'otherName'), '0.0.1')
        with self.assertRaisesRegex(ValueError, 'sensor'):
            validation.validate_manifest(manifest + ' android.permission.CAMERA', '0.0.1')

    def test_elf_checks_architecture_alignment_and_program_bounds(self):
        self.assertEqual(validation.validate_elf(elf()), [16384])
        wrong_class = elf()
        wrong_class[4] = 1
        invalid_offset = elf()
        struct.pack_into('<Q', invalid_offset, 32, 4096)
        for data in [b'not ELF', elf(machine=62), elf(alignment=4096), elf(alignment=20000),
                     wrong_class, invalid_offset]:
            with self.subTest(header=bytes(data[:20])):
                with self.assertRaises(ValueError):
                    validation.validate_elf(data)

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

        self.assertEqual(len(validation.validate_native(apk())), 5)
        for extra in ['lib/x86_64/libxrbox_godot.so', 'assets/debug.keystore', 'assets/tests/smoke.gdc', 'assets/native/libxrbox.dylib']:
            with self.subTest(extra=extra):
                with self.assertRaises(ValueError):
                    validation.validate_native(apk(extra=extra))
        with self.assertRaises(ValueError):
            validation.validate_native(apk(omit='libxrbox_godot.so'))


if __name__ == "__main__":
    unittest.main()
