#!/usr/bin/env python3
"""Exercise the actual heap launcher against small executable/Cargo fixtures."""

import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).resolve().parents[1] / "parallel-heaps.py"
spec = importlib.util.spec_from_file_location("parallel_heaps", SCRIPT)
heaps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(heaps)

CARGO = r'''#!/usr/bin/env python3
import json, os, pathlib, shlex, sys
root = pathlib.Path(os.environ['FIXTURE'])
args = sys.argv[1:]
if 'show-env' in args:
    profile = str(pathlib.Path(os.environ['CARGO_LLVM_COV_TARGET_DIR']) / os.environ['LLVM_PROFILE_FILE_NAME'])
    if os.environ.get('BAD_PROFILE'): profile = '/tmp/stale-%p-%m.profraw'
    for key, value in [('LLVM_PROFILE_FILE', profile), ('RUSTFLAGS', '-C instrument-coverage'), ('CARGO_LLVM_COV', '1')]:
        print('export ' + shlex.quote(key + '=' + value))
    sys.exit(0)
(root / 'cargo-args').write_text(json.dumps(args))
(root / 'cargo-env').write_text(json.dumps(dict(os.environ)))
names = ['heap_claude', 'heap_codex', 'heap_dsh']
mode = os.environ.get('ARTIFACTS')
if mode == 'missing': names.pop()
if mode == 'duplicate': names.append('heap_claude')
if mode == 'unexpected': names.append('it')
# Integration tests also cause Cargo to report the non-test CLI executable.
print(json.dumps({'reason': 'compiler-artifact', 'profile': {'test': False}, 'target': {'name': 'brokkr'}, 'executable': '/not/a/test'}))
for name in names:
    print(json.dumps({'reason': 'compiler-artifact', 'profile': {'test': True}, 'target': {'name': name}, 'executable': str(root / name)}))
'''

CHILD = r'''#!/usr/bin/env python3
import json, os, pathlib, signal, sys, time
root = pathlib.Path(os.environ['FIXTURE'])
name = pathlib.Path(sys.argv[0]).name
(root / (name + '.env')).write_text(json.dumps(dict(os.environ)))
(root / (name + '.args')).write_text(json.dumps(sys.argv[1:]))
if os.environ.get('IGNORE_TERM') == name: signal.signal(signal.SIGTERM, signal.SIG_IGN)
(root / (name + '.started')).write_text(str(os.getpid()))
# Every child must start before any can finish. A serial launcher fails.
deadline = time.monotonic() + 5
while len(list(root.glob('*.started'))) != 3:
    if time.monotonic() > deadline: sys.exit(90)
    time.sleep(.01)
if os.environ.get('HANG'):
    while True: time.sleep(.05)
if name == 'heap_claude':
    mode = os.environ.get('RESULT')
    if mode == 'signal': os.kill(os.getpid(), signal.SIGINT)
    if mode == 'failed': sys.exit(42)
    if mode == 'empty': sys.exit(0)
    if mode == 'zero':
        print('test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s')
        sys.exit(0)
    if mode == 'duplicate':
        print('test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s')
kind = {'heap_claude':'claude-session', 'heap_codex':'codex-thread', 'heap_dsh':'dsh-session'}[name]
if os.environ.get('RESULT') == 'wrong-kind' and name == 'heap_claude': kind = 'wrong'
print('test heap_budget ... heap\t' + kind + '\t123')
print('ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s')
(root / (name + '.finished')).touch()
'''


class Launcher(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="heap launcher ")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.env = os.environ.copy()
        self.env.update(FIXTURE=str(self.root), PATH=str(self.root) + os.pathsep + self.env['PATH'])
        for key in ('CARGO_LLVM_COV_TARGET_DIR', 'LLVM_PROFILE_FILE_NAME', 'LLVM_PROFILE_FILE'):
            self.env.pop(key, None)
        self.executable('cargo', CARGO)
        self.executable('rustc', '#!/bin/sh\ncase "$*" in *sysroot*) echo /toolchain ;; *) echo "host: fixture-host" ;; esac\n')
        for name in heaps.NAMES:
            self.executable(name, CHILD)

    def executable(self, name, body):
        path = self.root / name
        path.write_text(body)
        path.chmod(0o755)

    def command(self, *args):
        return [sys.executable, str(SCRIPT), *args]

    def launch(self, *args):
        return subprocess.run(self.command(*args), env=self.env, text=True, capture_output=True, timeout=20)

    def assert_reaped(self):
        for path in self.root.glob('*.started'):
            with self.assertRaises(ProcessLookupError, msg=str(path)):
                os.kill(int(path.read_text()), 0)

    def test_overlap_join_feature_selection_and_cargo_artifacts(self):
        result = self.launch()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(len(list(self.root.glob('*.finished'))), 3)
        self.assert_reaped()
        self.assertEqual(json.loads((self.root / 'cargo-args').read_text()), (
            'test --workspace --all-features --locked --no-fail-fast --message-format=json '
            '--test=heap_claude --test=heap_codex --test=heap_dsh --no-run').split())
        self.assertEqual(result.stdout.count('test result: ok.'), 3)
        for name in heaps.NAMES:
            self.assertEqual(json.loads((self.root / (name + '.args')).read_text()),
                             ['--nocapture', '--test-threads=1'])

    def test_coverage_preserves_gate_profile_target_and_instrumentation(self):
        target = self.root / 'coverage target'
        self.env.update(CARGO_LLVM_COV_TARGET_DIR=str(target), LLVM_PROFILE_FILE_NAME='current-run-%p-%m.profraw')
        result = self.launch('--nightly', 'nightly-2026-09-05')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(json.loads((self.root / 'cargo-args').read_text())[0], '+nightly-2026-09-05')
        for name in ['cargo', *heaps.NAMES]:
            file = 'cargo-env' if name == 'cargo' else name + '.env'
            env = json.loads((self.root / file).read_text())
            self.assertEqual(env['LLVM_PROFILE_FILE'], str(target / 'current-run-%p-%m.profraw'))
            self.assertEqual(env['CARGO_TARGET_DIR'], str(target))
            self.assertEqual(env['RUSTFLAGS'], '-C instrument-coverage')

    def test_coverage_without_the_gate_or_with_a_stale_profile_refuses(self):
        self.assertNotEqual(self.launch('--nightly', 'nightly-2026-09-05').returncode, 0)
        self.env.update(CARGO_LLVM_COV_TARGET_DIR=str(self.root), LLVM_PROFILE_FILE_NAME='current-%p-%m.profraw', BAD_PROFILE='1')
        self.assertNotEqual(self.launch('--nightly', 'nightly-2026-09-05').returncode, 0)
        self.assertFalse(list(self.root.glob('*.started')))

    def test_missing_duplicate_and_unexpected_test_artifacts_never_start(self):
        for mode in ('missing', 'duplicate', 'unexpected'):
            with self.subTest(mode=mode):
                self.env['ARTIFACTS'] = mode
                self.assertNotEqual(self.launch().returncode, 0)
                self.assertFalse(list(self.root.glob('*.started')))

    def test_failure_or_bad_results_still_join_the_other_two(self):
        for mode in ('failed', 'empty', 'zero', 'duplicate', 'wrong-kind', 'signal'):
            with self.subTest(mode=mode):
                for path in [*self.root.glob('*.started'), *self.root.glob('*.finished')]:
                    path.unlink()
                self.env['RESULT'] = mode
                result = self.launch()
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                for name in ('heap_codex', 'heap_dsh'):
                    self.assertTrue((self.root / (name + '.finished')).exists())
                self.assert_reaped()

    def test_cancellation_joins_every_child(self):
        for signum in (signal.SIGINT, signal.SIGTERM):
            with self.subTest(signal=signum):
                for path in self.root.glob('*.started'):
                    path.unlink()
                self.env['HANG'] = '1'
                process = subprocess.Popen(self.command(), env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                try:
                    deadline = time.monotonic() + 5
                    while len(list(self.root.glob('*.started'))) != 3:
                        self.assertIsNone(process.poll())
                        self.assertLess(time.monotonic(), deadline)
                        time.sleep(.01)
                    process.send_signal(signum)
                    process.communicate(timeout=10)
                    self.assertEqual(process.returncode, 128 + signum)
                    self.assert_reaped()
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate()

    def test_timeout_kills_and_reaps_even_a_child_ignoring_term(self):
        self.env.update(HANG='1', IGNORE_TERM='heap_codex')
        binaries = {name: str(self.root / name) for name in heaps.NAMES}
        with self.assertRaises(subprocess.TimeoutExpired):
            heaps.run(binaries, self.env, timeout=1)
        self.assertEqual(len(list(self.root.glob('*.started'))), 3)
        self.assert_reaped()


if __name__ == '__main__':
    unittest.main()
