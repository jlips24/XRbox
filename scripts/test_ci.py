"""Release-trigger and verified-download regressions; no network or tool installs."""
from pathlib import Path
import json
import tempfile
import unittest
from unittest.mock import patch

import ci


class CiTests(unittest.TestCase):
    def test_release_names_accept_only_two_or_three_numeric_components(self):
        for tag in ['v0.1', 'v1.2.3', 'v12.300.4', 'v1.99']:
            self.assertTrue(ci.valid_tag(tag), tag)
        for tag in ['1.2.3', 'v1', 'v1.2.3.4', 'v1.2-rc1', 'v1.2.3-beta',
                    'v1.2\n', 'v1.2\neligible=true', 'v１.２', 'v1.x', 'v1.2/branch']:
            self.assertFalse(ci.valid_tag(tag), repr(tag))

    def test_release_artifact_version_must_match_tag(self):
        ci.validate_release_version('v0.0.1', '0.0.1')
        ci.validate_release_version('v1.2', '1.2')
        for tag, app in [('v0.0.1', '0.1.10-xr.3'), ('v1.2', '1.2.0')]:
            with self.assertRaisesRegex(ValueError, 'APK version'):
                ci.validate_release_version(tag, app)

    def test_tag_gate_writes_no_untrusted_output_fields(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'outputs'
            for tag, expected in [('v1.2', 'eligible=true\ntag=v1.2\n'),
                                  ('v1.2\nother=bad', 'eligible=false\n')]:
                output.write_text('')
                with patch.dict(ci.os.environ, {'GITHUB_ACTIONS': 'true', 'XRBOX_TAG': tag,
                                                'GITHUB_OUTPUT': str(output)}), \
                        patch('sys.argv', ['ci.py', 'tag']):
                    ci.main()
                self.assertEqual(output.read_text(), expected)

    def test_android_metadata_records_target_separately_from_build_host(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'xr').mkdir()
            (root / 'xr/project.godot').write_text('config/version="0.0.1"\n')
            core = root / 'crates/xrbox-core'
            core.mkdir(parents=True)
            (core / 'Cargo.toml').write_text('version = "0.1.10"\n')
            artifact = root / 'artifact'
            artifact.mkdir()
            (artifact / 'test.apk').write_bytes(b'synthetic APK')
            with patch.object(ci, 'ROOT', root), \
                    patch.object(ci.platform, 'machine', return_value='x86_64'), \
                    patch.object(ci.subprocess, 'check_output', return_value='source-commit\n'), \
                    patch.dict(ci.os.environ, {'XRBOX_TAG': 'v0.0.1', 'XRBOX_BUILD_KIND': 'android-development-apk'}):
                ci.metadata(artifact)
            info = json.loads((artifact / 'build.json').read_text())
            self.assertEqual(info['architecture'], 'aarch64')
            self.assertEqual(info['target'], 'aarch64-linux-android')
            self.assertEqual(info['build_host_architecture'], 'x86_64')
            self.assertEqual(info['app_version'], '0.0.1')
            self.assertFalse(info['hardware_validated'])
            self.assertEqual(info['sha256']['test.apk'], ci.digest(artifact / 'test.apk', 'sha256'))

    def test_verified_cache_is_reused_but_mismatched_download_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'archive.zip'
            path.write_bytes(b'verified')
            pin = {'url': 'https://unused.invalid', 'sha512': ci.digest(path, 'sha512')}
            with patch.object(ci.urllib.request, 'urlopen') as network:
                self.assertEqual(ci.download(pin, path), path)
                network.assert_not_called()
            import io
            path.unlink()
            with patch.object(ci.urllib.request, 'urlopen', return_value=io.BytesIO(b'tampered')):
                with self.assertRaisesRegex(ValueError, 'checksum'):
                    ci.download(pin, path)
            self.assertFalse(path.exists())
            self.assertFalse(path.with_suffix('.download').exists())


if __name__ == '__main__':
    unittest.main()
