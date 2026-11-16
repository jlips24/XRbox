#!/usr/bin/env python3
"""Single source for app/APK versions; synchronize derived Godot settings."""
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def load(root=ROOT):
    config = json.loads((root / 'release.json').read_text())
    if not isinstance(config, dict) or set(config) != {'version', 'android_version_code'}:
        raise ValueError('release.json requires only version and android_version_code')
    version, code = config['version'], config['android_version_code']
    if not isinstance(version, str) or re.fullmatch(r'[0-9]+\.[0-9]+(?:\.[0-9]+)?', version, re.ASCII) is None:
        raise ValueError('Release version must use X.X or X.X.X numeric components')
    if type(code) is not int or not 1 <= code <= 2100000000:
        raise ValueError('Android version code must be an integer from 1 to 2100000000')
    return config


def apk_name(config):
    return 'XRbox-' + config['version'] + '-quest-debug.apk'


def replace_setting(text, section, key, value):
    current, found, lines = None, 0, []
    for line in text.splitlines(keepends=True):
        if line.strip().startswith('['):
            current = line.strip()
        if current == '[' + section + ']' and re.match(r'^' + re.escape(key) + r'\s*=', line):
            line = key + '=' + str(value) + '\n'
            found += 1
        lines.append(line)
    if found != 1:
        raise ValueError(f'Expected one {section}/{key} setting; found {found}')
    return ''.join(lines)


def sync(root=ROOT):
    config = load(root)
    version = json.dumps(config['version'])
    updates = []
    for relative, settings in [
            ('xr/project.godot', [('application', 'config/version', version)]),
            ('xr/export_presets.cfg', [('preset.1', 'export_path', json.dumps('../dist/xr/android/' + apk_name(config))),
                                       ('preset.1.options', 'version/name', version),
                                       ('preset.1.options', 'version/code', config['android_version_code'])])]:
        path = root / relative
        original = text = path.read_text()
        for section, key, value in settings:
            text = replace_setting(text, section, key, value)
        updates.append((path, original, text))
    # Validate all expected settings before writing either file.
    for path, original, text in updates:
        if text != original:
            path.write_text(text)
    return config


if __name__ == '__main__':
    sync()
