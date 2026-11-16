"""Release config validation and end-to-end synchronization; no builds or devices."""
import json
from pathlib import Path
import tempfile
import unittest

import release_version


class ReleaseVersionTests(unittest.TestCase):
    def test_bump_updates_app_and_apk_without_altering_other_settings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'xr').mkdir()
            for name in ['project.godot', 'export_presets.cfg']:
                (root / 'xr' / name).write_text((release_version.ROOT / 'xr' / name).read_text())
            (root / 'release.json').write_text(json.dumps(dict(version='0.0.2', android_version_code=11004)))
            config = release_version.sync(root)
            self.assertEqual(release_version.apk_name(config), 'XRbox-0.0.2-quest-debug.apk')
            project, preset = root / 'xr/project.godot', root / 'xr/export_presets.cfg'
            self.assertIn('config/version="0.0.2"', project.read_text())
            self.assertIn('version/name="0.0.2"', preset.read_text())
            self.assertIn('version/code=11004', preset.read_text())
            self.assertIn('../dist/xr/android/XRbox-0.0.2-quest-debug.apk', preset.read_text())
            def nonversion_lines(text):
                return [line for line in text.splitlines() if not line.startswith(
                    ('config/version=', 'version/name=', 'version/code=', 'export_path="../dist/xr/android/'))]
            for path in [project, preset]:
                self.assertEqual(nonversion_lines(path.read_text()),
                                 nonversion_lines((release_version.ROOT / 'xr' / path.name).read_text()))
            times = (project.stat().st_mtime_ns, preset.stat().st_mtime_ns)
            release_version.sync(root)
            self.assertEqual(times, (project.stat().st_mtime_ns, preset.stat().st_mtime_ns))

    def test_invalid_config_cannot_partially_update_project(self):
        cases = [dict(version='v0.0.2', android_version_code=11004),
                 dict(version='0.0.2\nextra', android_version_code=11004),
                 dict(version='0.0.2', android_version_code=True),
                 dict(version='0.0.2', android_version_code=0),
                 dict(version='0.0.2', android_version_code=2100000001),
                 dict(version='0.0.2', android_version_code='11004'),
                 dict(version='0.0.2')]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for config in cases:
                with self.subTest(config=config):
                    (root / 'release.json').write_text(json.dumps(config))
                    with self.assertRaises(ValueError):
                        release_version.sync(root)
                    self.assertFalse((root / 'xr').exists())

    def test_missing_preset_setting_leaves_project_unmodified(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'xr').mkdir()
            (root / 'release.json').write_text(json.dumps(dict(version='1.2', android_version_code=12000)))
            project = root / 'xr/project.godot'
            original = '[application]\nconfig/version="0.0.1"\n'
            project.write_text(original)
            (root / 'xr/export_presets.cfg').write_text('[preset.1]\n')
            with self.assertRaisesRegex(ValueError, 'Expected one'):
                release_version.sync(root)
            self.assertEqual(project.read_text(), original)


if __name__ == '__main__':
    unittest.main()
