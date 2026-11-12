#!/usr/bin/env python3
"""Run every automated suite with bounded terminal output and retained full logs."""
import argparse
from datetime import datetime
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
SUITES = (
    ('Rust workspace', 'test-rust-all'),
    ('Python helpers', 'test-python'),
    ('Godot frontend', 'xr-check'),
    ('GPU render smoke', 'xr-render'),
)


def run_suites(suites, directory, cwd=ROOT):
    """Continue after failures; exit status reflects every suite, never log parsing."""
    results = []
    print(f'Full output: {directory}', flush=True)
    for index, (name, command) in enumerate(suites, 1):
        log = directory / f'{index:02d}-{name.lower().replace(" ", "-")}.log'
        print(f'[{index}/{len(suites)}] Running {name}…', flush=True)
        started = time.monotonic()
        interrupted = False
        with log.open('w', encoding='utf-8') as output:
            try:
                code = subprocess.run(command, cwd=cwd, stdout=output,
                                      stderr=subprocess.STDOUT, check=False).returncode
            except OSError as error:
                output.write(f'Could not start suite: {error}\n')
                code = 127
            except KeyboardInterrupt:
                output.write('\nSuite interrupted by user.\n')
                code = 130
                interrupted = True
        status = 'INTERRUPTED' if interrupted else ('PASS' if code == 0 else 'FAIL')
        results.append((name, status, time.monotonic() - started, code, log))
        print(f'      {status} ({results[-1][2]:.1f}s)', flush=True)
        if interrupted:
            break
    print('\nTest summary')
    for name, status, seconds, code, log in results:
        detail = '' if code == 0 else f' (exit {code})'
        print(f'  {status:<11} {name:<18} {seconds:>7.1f}s{detail}')
    if len(results) < len(suites):
        print(f'  NOT RUN     {len(suites) - len(results)} remaining suite(s)')
    print(f'\nLogs: {directory}')
    for name, status, _, _, log in results:
        if status != 'PASS':
            print(f'  {name}: {log}')
    if any(status == 'INTERRUPTED' for _, status, *_ in results):
        return 130
    return 1 if any(code != 0 for _, _, _, code, _ in results) else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--make', default='make', help='Make executable (no shell evaluation)')
    args = parser.parse_args()
    parent = ROOT / '.tools' / 'test-results'
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix=datetime.now().strftime('%Y%m%d-%H%M%S-'), dir=parent))
    suites = [(name, [args.make, '--no-print-directory', target]) for name, target in SUITES]
    return run_suites(suites, directory)


if __name__ == '__main__':
    raise SystemExit(main())
