"""Packaging regressions: reject wrong ABI, malformed ELF and unsafe dependencies."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import package_android as packaging


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
