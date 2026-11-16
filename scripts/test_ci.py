"""Release-trigger and verified-download regressions; no network or tool installs."""
from pathlib import Path
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
