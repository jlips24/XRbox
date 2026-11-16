"""Flatpak source isolation, exported-package contract and failure regressions."""
import configparser
import json
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import package_flatpak as packaging


class FlatpakTests(unittest.TestCase):
    def test_manifest_and_desktop_entry_launch_the_exported_app(self):
        root = packaging.ROOT
        manifest = json.loads((root / 'packaging/flatpak' / (packaging.APP_ID + '.json')).read_text())
        desktop = configparser.ConfigParser(interpolation=None)
        desktop.read(root / 'packaging/flatpak' / (packaging.APP_ID + '.desktop'))
        metadata = ET.parse(root / 'packaging/flatpak' / (packaging.APP_ID + '.metainfo.xml')).getroot()
        pins = json.loads((root / 'xr/toolchain.json').read_text())
        self.assertEqual(manifest['app-id'], packaging.APP_ID)
        self.assertEqual(manifest['runtime-version'], pins['flatpak']['runtime_version'])
        self.assertEqual(desktop['Desktop Entry']['Exec'], manifest['command'])
        self.assertEqual(metadata.findtext('id'), packaging.APP_ID)
        self.assertEqual(metadata.findtext('launchable'), packaging.APP_ID + '.desktop')
        self.assertNotIn('--filesystem=host', manifest['finish-args'])
        self.assertNotIn('--filesystem=home', manifest['finish-args'])
        self.assertNotIn('--socket=session-bus', manifest['finish-args'])
        self.assertEqual(packaging.bundle_name('0.0.4'), 'XRbox-0.0.4-linux-arm64.flatpak')

    def test_staging_excludes_ignored_credentials_outputs_and_git_database(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, dest = Path(temporary) / 'repo', Path(temporary) / 'stage'
            root.mkdir()
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            (root / '.gitignore').write_text('.tools/\ndist/\n*.keystore\n')
            for name in ['crates/core.rs', '.tools/debug.keystore', 'dist/app.flatpak', 'local.keystore']:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('synthetic')
            dest.mkdir()
            with patch.object(packaging, 'ROOT', root):
                packaging.copy_sources(dest)
            self.assertTrue((dest / 'crates/core.rs').is_file())
            self.assertFalse((dest / '.tools').exists())
            self.assertFalse((dest / 'dist').exists())
            self.assertFalse((dest / 'local.keystore').exists())
            self.assertFalse((dest / '.git').exists())
            (root / 'escape').symlink_to(root / 'local.keystore')
            with patch.object(packaging, 'ROOT', root), self.assertRaisesRegex(ValueError, 'symlinks'):
                packaging.copy_sources(dest)

    def test_export_requires_app_resources_and_matching_arm64_native_library(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            elf = bytearray(20)
            elf[:6] = b'\x7fELF\x02\x01'
            elf[18:20] = (183).to_bytes(2, 'little')
            (directory / 'libxrbox_godot.so').write_bytes(elf)
            with self.assertRaisesRegex(ValueError, 'Incomplete'):
                packaging.validate_export(directory)
            (directory / 'xrbox.pck').write_bytes(b'synthetic PCK')
            (directory / 'xrbox.arm64').write_bytes(elf)
            packaging.validate_export(directory)
            elf[18:20] = (62).to_bytes(2, 'little')
            (directory / 'libxrbox_godot.so').write_bytes(elf)
            with self.assertRaisesRegex(ValueError, 'ARM64 ELF'):
                packaging.validate_export(directory)

    def test_godot_errors_fail_even_with_zero_process_exit(self):
        packaging.check_output('Synthetic smoke completed')
        for output in ['ERROR: GDExtension failed to load', 'SCRIPT ERROR: Parse error']:
            with self.assertRaises(ValueError):
                packaging.check_output(output)

    def test_missing_elfutils_fails_before_staging_or_downloading(self):
        for missing in ['eu-strip', 'eu-elfcompress']:
            with self.subTest(missing=missing), \
                    patch.object(packaging.platform, 'system', return_value='Linux'), \
                    patch.object(packaging.platform, 'machine', return_value='aarch64'), \
                    patch.object(packaging.shutil, 'which', side_effect=lambda tool: None if tool == missing else '/synthetic/' + tool), \
                    patch.object(packaging, 'copy_sources') as staging, \
                    patch.object(packaging.ci, 'download') as download, \
                    self.assertRaisesRegex(ValueError, 'Missing prerequisite: ' + missing):
                packaging.build()
            staging.assert_not_called()
            download.assert_not_called()

    def test_ci_build_failure_survives_tee_and_retains_log(self):
        workflow = (packaging.ROOT / '.github/workflows/build.yml').read_text()
        step = workflow.split('      - name: Build and verify the installable Flatpak\n', 1)[1].split('      - name:', 1)[0]
        self.assertIn('        shell: bash\n', step)
        script = textwrap.dedent(step.split('        run: |\n', 1)[1])
        # Run the actual workflow body under GitHub's explicit bash flags.
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run(
                ['bash', '--noprofile', '--norc', '-eo', 'pipefail', '-c',
                 "make() { printf 'synthetic Flatpak failure\\n'; return 23; }\n" + script],
                cwd=temporary, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            self.assertEqual(result.returncode, 23, result.stdout)
            self.assertIn('synthetic Flatpak failure', (Path(temporary) / '.tools/flatpak-build.log').read_text())

    def test_unsupported_host_fails_before_downloading_or_building(self):
        with patch.object(packaging.platform, 'system', return_value='Darwin'), \
                patch.object(packaging.ci, 'download') as download, \
                self.assertRaisesRegex(ValueError, 'Linux ARM64'):
            packaging.build()
        download.assert_not_called()


if __name__ == '__main__':
    unittest.main()
