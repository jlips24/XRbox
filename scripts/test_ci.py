"""Release-trigger and verified-download regressions; no network or tool installs."""
from pathlib import Path
import json
import tempfile
import textwrap
import subprocess
import unittest
from unittest.mock import patch

import ci


class CiTests(unittest.TestCase):
    def test_lint_workflow_preserves_exit_status_and_captured_output(self):
        workflow = (ci.ROOT / '.github/workflows/test.yml').read_text()
        step = workflow.split('      - name: Formatting and full workspace lint\n', 1)[1].split('      - name:', 1)[0]
        self.assertIn('        shell: bash\n', step)
        script = textwrap.dedent(step.split('        run: |\n', 1)[1])
        # GitHub supplies pipefail when the workflow explicitly selects bash.
        for code in [0, 23]:
            with self.subTest(exit_code=code), tempfile.TemporaryDirectory() as temporary:
                fake_make = "make() { printf 'synthetic lint output\\n'; return %d; }\n" % code
                result = subprocess.run(
                    ['bash', '--noprofile', '--norc', '-eo', 'pipefail', '-c', fake_make + script],
                    cwd=temporary, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
                self.assertEqual(result.returncode, code, result.stdout)
                self.assertIn('synthetic lint output', result.stdout)
                self.assertIn('synthetic lint output',
                              (Path(temporary) / '.tools/test-results/check-all.log').read_text())

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
            (root / 'release.json').write_text(json.dumps(dict(version='0.0.1', android_version_code=11003)))
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
            self.assertEqual(info['android_version_code'], 11003)
            self.assertFalse(info['hardware_validated'])
            self.assertEqual(info['sha256']['test.apk'], ci.digest(artifact / 'test.apk', 'sha256'))

    def test_release_source_requires_matching_version_and_master_ancestry(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'xr').mkdir()
            (root / 'release.json').write_text(json.dumps(dict(version='0.0.1', android_version_code=11003)))
            def git(*arguments):
                return subprocess.check_output(['git', '-c', 'user.name=XRbox test', '-c',
                                                'user.email=test@example.invalid', *arguments], cwd=root, text=True).strip()
            git('init', '-b', 'master')
            git('commit', '--allow-empty', '-m', 'synthetic master')
            revision = git('rev-parse', 'HEAD')
            git('update-ref', 'refs/remotes/origin/master', revision)
            with patch.object(ci, 'ROOT', root):
                self.assertEqual(ci.release_source('v0.0.1'), revision)
                with self.assertRaises(ValueError):
                    ci.release_source('v0.0.2')
                git('switch', '-c', 'feat/unmerged')
                git('commit', '--allow-empty', '-m', 'unmerged synthetic change')
                with self.assertRaises(subprocess.CalledProcessError):
                    ci.release_source('v0.0.1')

    def test_release_assets_verify_both_builds_before_packaging(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            linux, android = root / 'linux', root / 'android'
            for folder, kind, files in [
                    (linux, 'linux-arm64-flatpak', ['XRbox-0.0.1-linux-arm64.flatpak']),
                    (android, 'android-development-apk', ['XRbox-0.0.1-quest-debug.apk'])]:
                folder.mkdir()
                for name in files + ['LICENSE', 'THIRD_PARTY_NOTICES.md']:
                    (folder / name).write_text('synthetic ' + name)
                info = dict(tag='v0.0.1', source_revision='abc', kind=kind, app_version='0.0.1',
                            sha256={p.name: ci.digest(p, 'sha256') for p in folder.iterdir()})
                (folder / 'build.json').write_text(json.dumps(info))
            output = root / 'assets'
            ci.release_assets(linux, android, output, 'v0.0.1', 'abc')
            self.assertEqual({p.name for p in output.iterdir()},
                             {'XRbox-0.0.1-linux-arm64.flatpak', 'XRbox-0.0.1-quest-debug.apk'})
            self.assertEqual((output / 'XRbox-0.0.1-linux-arm64.flatpak').read_bytes(),
                             (linux / 'XRbox-0.0.1-linux-arm64.flatpak').read_bytes())
            for revision, tag in [('wrong-source', 'v0.0.1'), ('abc', 'v0.0.2')]:
                with self.assertRaisesRegex(ValueError, 'provenance'):
                    ci.release_assets(linux, android, root / 'rejected', tag, revision)
                self.assertFalse((root / 'rejected').exists())
            (android / 'XRbox-0.0.1-quest-debug.apk').write_text('tampered')
            with self.assertRaisesRegex(ValueError, 'checksum'):
                ci.release_assets(linux, android, root / 'rejected', 'v0.0.1', 'abc')
            self.assertFalse((root / 'rejected').exists())

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
