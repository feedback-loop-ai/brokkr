#!/usr/bin/env python3
"""Exercise the foreground shard runner against small executable fixtures.

The runner owns its child's output and process group, so these tests drive
the real script as a subprocess: a bound that fires, an exit on the child's
own, a signal forwarded, a record that cannot be written and a group that
cannot be shown ended. A test binary is a real executable copied into a
directory named `deps`, so the kernel's own executable path is what the
runner reads, never a line the binary prints. The process-table readers are
unit-tested against planted /proc trees and a stub ps, since a test cannot
make the host's own table unreadable.
"""

import contextlib
import importlib.util
import io
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

sys.dont_write_bytecode = True
SCRIPT = Path(__file__).resolve().parents[1] / "shard-runner.py"
SHELL = "/bin/sh"


def load_runner():
    spec = importlib.util.spec_from_file_location("shard_runner", SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {SCRIPT}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def stat_line(pid, group, start):
    """A /proc/<pid>/stat line whose field 22 is `start`."""
    fields = ["S", "1", str(group), str(group), "0", "-1", "0", "0", "0", "0",
              "0", "0", "0", "0", "0", "20", "0", "1", "0", str(start)]
    return f"{pid} (member) {' '.join(fields)}\n"


class ShardRunner(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="shard runner ")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.status = self.root / "status"
        self.record = self.root / "bound"
        self.runner = load_runner()

    def child(self, body, name="child"):
        path = self.root / name
        path.write_text("#!/bin/sh\n" + body)
        path.chmod(0o755)
        return path

    def deps(self, name, folder="target/debug/deps"):
        """A real executable copied to `<folder>/<name>-0123abcd`.

        `shutil.copy` takes the bytes and the mode, never the flags: macOS
        protects `/bin/sh` with a system flag no copy may carry, so
        `copy2`'s flag copy is refused there. Every body run by a copy ends
        in the builtin `:`, so no shell runs its last command by exec in
        its own place, as macOS's `/bin/sh` does, which would make the copy
        a `sleep` outside `deps` before the runner first saw it.
        """
        directory = self.root / folder
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"{name}-0123abcd"
        shutil.copy(SHELL, path)
        path.chmod(0o755)
        return path

    def deps_argv(self, name, body, *args, folder="target/debug/deps"):
        path = self.deps(name, folder)
        return [path, "-c", body, name, *map(str, args)]

    def command(self, argv, bound=1800, record=None, status=None):
        return [sys.executable, str(SCRIPT),
                "--binary-bound", str(bound),
                "--bound-record", str(record or self.record),
                "--status", str(status or self.status),
                "--", *map(str, argv)]

    def launch(self, argv, bound=1800, record=None, status=None, env=None):
        environment = os.environ.copy()
        environment.pop("BROKKR_TEST_SHARD_BOUND_SECONDS", None)
        if env:
            environment.update(env)
        return subprocess.run(self.command(argv, bound, record, status), env=environment,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=40)

    def pid(self, name="child.pid"):
        path = self.root / name
        deadline = time.monotonic() + 10
        while not path.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        return int(path.read_text().strip())

    def kill_group(self, pid):
        try:
            os.killpg(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

    def killed_after(self, pid, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return True
            time.sleep(0.02)
        return False

    def test_a_deps_binary_past_the_bound_is_named_ended_and_recorded(self):
        pidfile = self.root / "hang.pid"
        argv = self.deps_argv(
            "hang", 'printf "%s\\n" "$$" > "$1"; sleep 600; :', pidfile)
        started = time.monotonic()
        result = self.launch(argv, bound=1)
        elapsed = time.monotonic() - started
        # Under the bound plus the 10 s TERM grace a group that ignored TERM
        # would take, with room for a loaded host (#1054).
        self.assertLess(elapsed, 9, result.stdout + result.stderr)
        self.assertGreaterEqual(elapsed, 1.0, "judged before its bound")
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "hang-0123abcd\n1\n")
        self.assertEqual(result.stderr, b"")
        self.assertTrue(self.killed_after(self.pid("hang.pid")))

    def test_a_binary_that_ends_on_its_own_records_its_exit(self):
        result = self.launch(self.deps_argv("exits", "exit 3"), bound=1)
        self.assertEqual(result.returncode, 3, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "exit 3\n")
        self.assertFalse(self.record.exists())

    def test_a_failed_group_read_during_a_poll_is_unproven_not_an_exit(self):
        child = self.child("sleep 3\n", "quiet")
        calls = []

        def flaky(_pgid):
            calls.append(1)
            if len(calls) == 1:
                return None, "the process table could not be read"
            return [], None

        runner = self.runner.Runner([str(child)], 1800, str(self.record), str(self.status),
                                    read_group=flaky)
        code = runner.run()
        self.assertEqual(code, 125)
        self.assertEqual(self.status.read_text(),
                         "unproven the process table could not be read\n")

    def test_a_planted_proc_failure_during_a_poll_is_unproven(self):
        proc = self.root / "planted-proc"
        (proc / "1").mkdir(parents=True)
        (proc / "1" / "stat").write_text(stat_line(1, 7, 99))
        (proc / "2").mkdir()
        (proc / "2" / "stat").write_text(stat_line(2, 8, 100))
        original = Path.iterdir
        failed = []

        def iterdir(path):
            if path == proc and not failed:
                failed.append(1)
                shutil.rmtree(proc / "1")
                raise PermissionError(13, "the process table is not readable")
            return original(path)

        runner = self.runner.Runner(
            [str(self.child("sleep 3\n", "planted"))], 1800, str(self.record),
            str(self.status), read_group=lambda pgid: self.runner.proc_group(pgid, root=proc))
        with mock.patch.object(Path, "iterdir", iterdir):
            code = runner.run()
        self.assertEqual(failed, [1], "the planted failure was never read")
        self.assertEqual(code, 125)
        self.assertEqual(self.status.read_text(),
                         "unproven the process table could not be read\n")

    def test_a_stub_ps_failing_during_a_poll_is_unproven_under_darwin(self):
        stub = self.root / "bin"
        stub.mkdir()
        ps = stub / "ps"
        ps.write_text(
            "#!/bin/sh\n"
            'marker="$STUB_PS_MARKER"\n'
            'if [ ! -e "$marker" ]; then : > "$marker"; exit 1; fi\n'
            "printf '   99    99 Sun Nov  9 01:23:25 2025 unrelated\\n'\n")
        ps.chmod(0o755)
        marker = self.root / "ps-failed"
        old = os.environ.get("PATH", "")
        os.environ["PATH"] = f"{stub}{os.pathsep}{old}"
        os.environ["STUB_PS_MARKER"] = str(marker)
        try:
            runner = self.runner.Runner(
                [str(self.child("sleep 3\n", "darwin-ps"))], 1800, str(self.record),
                str(self.status))
            with mock.patch.object(self.runner.platform, "system", return_value="Darwin"):
                code = runner.run()
        finally:
            os.environ["PATH"] = old
            os.environ.pop("STUB_PS_MARKER", None)
        self.assertTrue(marker.exists(), "the stub ps was never asked")
        self.assertEqual(code, 125)
        self.assertEqual(self.status.read_text(),
                         "unproven the process table could not be read\n")

    def test_a_member_left_after_the_child_ends_is_unproven_and_named(self):
        member = 4321
        for system, reported in [("Linux", f" {member}"), ("Darwin", "")]:
            with self.subTest(system=system):
                self.status.unlink(missing_ok=True)
                name = f"leaves-{system}"
                child = self.child(
                    "sleep 60 &\n"
                    'printf "%s %s\\n" "$$" "$!" > "$1"\n'
                    "exit 0\n", name)
                pidfile = self.root / f"{name}.pid"
                leader = None
                try:
                    with mock.patch.object(self.runner.platform, "system", return_value=system):
                        code = self.runner.Runner(
                            [str(child), str(pidfile)], 1800, str(self.record),
                            str(self.status),
                            read_group=lambda _pgid: (
                                [self.runner.Member(member, 0, None)], None)).run()
                    leader = int(pidfile.read_text().split()[0])
                    self.assertEqual(code, 125)
                    self.assertEqual(self.status.read_text(),
                                     f"unproven process group {leader} still has members{reported}\n")
                finally:
                    if leader is not None:
                        try:
                            self.kill_group(leader)
                        except ProcessLookupError:
                            pass

    def test_a_killed_runner_leaves_no_status(self):
        pidfile = self.root / "killed.pid"
        argv = self.deps_argv(
            "killed", 'printf "%s\\n" "$$" > "$1"; sleep 600; :', pidfile)
        runner = subprocess.Popen(self.command(argv), stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE)
        try:
            self.pid("killed.pid")
            runner.kill()
            runner.communicate(timeout=10)
            self.assertFalse(self.status.exists())
        finally:
            if runner.poll() is None:
                runner.kill()
                runner.communicate()
            try:
                self.kill_group(self.pid("killed.pid"))
            except (ProcessLookupError, ValueError):
                pass

    def test_a_record_that_cannot_be_written_still_ends_the_group(self):
        pidfile = self.root / "record.pid"
        argv = self.deps_argv(
            "record", 'printf "%s\\n" "$$" > "$1"; sleep 600; :', pidfile)
        try:
            result = self.launch(argv, bound=1,
                                 record=self.root / "missing" / "bound")
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertFalse(self.status.exists())
            self.assertTrue(self.killed_after(self.pid("record.pid")),
                            "the record's failure left the group running")
        finally:
            try:
                self.kill_group(self.pid("record.pid"))
            except (ProcessLookupError, ValueError):
                pass

    def test_a_passing_result_line_cannot_clear_a_binarys_clock(self):
        argv = self.deps_argv(
            "tail",
            "printf 'test result: ok. 1 passed; 0 failed; 0 ignored; "
            "0 measured; 0 filtered out; finished in 0.20s\\n'\n"
            "sleep 10; :\n")
        result = self.launch(argv, bound=1)
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "tail-0123abcd\n1\n")

    def test_progress_output_does_not_move_the_deadline(self):
        argv = self.deps_argv(
            "chatty",
            "count=0\n"
            "while [ \"$count\" -lt 20 ]; do\n"
            "  sleep 0.3\n"
            "  printf 'test still running...\\n'\n"
            "  count=$((count + 1))\n"
            "done\n"
            "printf 'test result: ok. 1 passed; 0 failed; 0 ignored; "
            "0 measured; 0 filtered out; finished in 6.00s\\n'\n"
            "sleep 1; :\n")
        started = time.monotonic()
        result = self.launch(argv, bound=1)
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "chatty-0123abcd\n1\n")
        # Progress for 6 s: a deadline it moved would fire at 7 s at the
        # earliest, well past this ceiling and a loaded host's lag (#1054).
        self.assertLess(elapsed, 5, result.stdout + result.stderr)

    def test_a_deps_path_with_parentheses_is_named_and_bounded(self):
        pidfile = self.root / "parens.pid"
        argv = self.deps_argv(
            "hang", 'printf "%s\\n" "$$" > "$1"; sleep 30; :', pidfile,
            folder="target/ctd (y)/debug/deps")
        started = time.monotonic()
        result = self.launch(argv, bound=2)
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "hang-0123abcd\n2\n")
        self.assertLess(elapsed, 9, result.stdout + result.stderr)
        self.assertTrue(self.killed_after(self.pid("parens.pid")))

    def test_a_test_binary_that_prints_no_start_line_is_still_bounded(self):
        pidfile = self.root / "quiet.pid"
        argv = self.deps_argv(
            "quiet", 'printf "%s\\n" "$$" > "$1"; sleep 5; :', pidfile)
        result = self.launch(argv, bound=1)
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "quiet-0123abcd\n1\n")
        self.assertEqual(result.stdout, b"")

    def test_start_lines_printing_every_quarter_second_cannot_restart_a_clock(self):
        pidfile = self.root / "hang.pid"
        dep = self.deps("hang")
        cargo = self.child(
            "deps=\"$1\"\n"
            "(\n"
            "  count=0\n"
            "  while [ \"$count\" -lt 12 ]; do\n"
            "    printf '     Running unittests src/main.rs "
            "(target/debug/deps/hang-0123abcd)\\n'\n"
            "    sleep 0.4\n"
            "    count=$((count + 1))\n"
            "  done\n"
            "  printf 'test result: ok. 1 passed; 0 failed; 0 ignored; "
            "0 measured; 0 filtered out; finished in 4.80s\\n'\n"
            ") &\n"
            "\"$deps\" -c 'printf \"%s\\n\" \"$$\" > \"$1\"; sleep 10; :' hang \"$2\"\n"
            "wait\n", "cargo")
        started = time.monotonic()
        result = self.launch([cargo, dep, pidfile], bound=1)
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "hang-0123abcd\n1\n")
        self.assertGreaterEqual(elapsed, 1.0, "judged before its bound")
        # A clock the last Running line reset would fire at 5.4 s (#1054).
        self.assertLess(elapsed, 4, result.stdout + result.stderr)
        self.assertTrue(self.killed_after(self.pid("hang.pid")))

    def test_an_ansi_decorated_start_line_starts_no_clock(self):
        dep = self.deps("color")
        cargo = self.child(
            "printf '\\033[1m\\033[92m     Running\\033[0m unittests src/lib.rs "
            "(target/debug/deps/color-0123abcd)\\n'\n"
            "\"$1\" -c 'sleep 10; :'\n", "cargo-color")
        result = self.launch([cargo, dep], bound=1)
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "color-0123abcd\n1\n")
        self.assertIn(b"\x1b[1m\x1b[92m     Running\x1b[0m", result.stdout)

    def test_cargos_own_time_is_not_a_binarys(self):
        dep = self.deps("short")
        cargo = self.child(
            "sleep 3\n"
            "\"$1\" -c 'sleep 0.2; :'\n"
            "exit 0\n", "cargo-slow")
        started = time.monotonic()
        # 2 s: under cargo's own 3 s, over the 0.2 s binary on a loaded host.
        result = self.launch([cargo, dep], bound=2)
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "exit 0\n")
        self.assertFalse(self.record.exists(), "cargo's own time was judged")
        self.assertGreaterEqual(elapsed, 3.0, "the fake cargo did not compile")

    def test_a_doctest_past_the_bound_is_named_by_its_crate(self):
        rustdoc = self.root / "rustdoc"
        shutil.copy(SHELL, rustdoc)
        rustdoc.chmod(0o755)
        started = time.monotonic()
        result = self.launch(
            [rustdoc, "-c", "sleep 30; :", "rustdoc", "--crate-name", "brokkr_core",
             self.runner.DOCTEST_FLAG], bound=2)
        elapsed = time.monotonic() - started
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "Doc-tests brokkr_core\n2\n")
        self.assertGreaterEqual(elapsed, 2.0, "judged before its bound")
        self.assertLess(elapsed, 9, result.stdout + result.stderr)

    def test_test_binary_names_follow_the_process_rules(self):
        member = self.runner.Member
        long_name = "x" * 250
        cases = [
            (member(1, 0, "/tmp/target/debug/deps/abc-1234"), None, "abc-1234"),
            (member(2, 0, f"/tmp/target/debug/deps/{long_name}"), None, long_name[:200]),
            (member(3, 0, "/tmp/target/aarch64/debug/deps/arm-5678"), None, "arm-5678"),
            (member(4, 0, "/tmp/target/llvm-cov-target/debug/deps/cov-9012"), None,
             "cov-9012"),
            (member(5, 0, "/usr/bin/rustdoc"), ["rustdoc", self.runner.DOCTEST_FLAG,
                                                "--crate-name", "brokkr_core"],
             "Doc-tests brokkr_core"),
            (member(6, 0, "/usr/bin/rustdoc"), ["rustdoc", self.runner.DOCTEST_FLAG],
             "Doc-tests 6"),
            (member(7, 0, "/usr/bin/rustdoc"), None, "Doc-tests 7"),
            (member(8, 0, "/usr/bin/rustdoc"), ["rustdoc", "--crate-name", "x"], None),
            (member(9, 0, "/usr/bin/cargo"), None, None),
            (member(10, 0, "/tmp/target/debug/build/x"), None, None),
            (member(12, 0, "/bin/sleep"), None, None),
        ]
        for entry, args, expected in cases:
            with self.subTest(pid=entry.pid):
                self.assertEqual(
                    self.runner.test_binary_name(entry, read=lambda _pid: args), expected)
        unreadable = member(11, 0, None, "a member's executable could not be read")
        self.assertEqual(
            self.runner.judged_name(unreadable),
            (None, "a member's executable could not be read"))

    def test_a_clock_is_first_sight_plus_the_bound_and_never_moves(self):
        runner = self.runner.Runner(["unused"], 4, str(self.record), str(self.status))
        clocks = {}
        first = self.runner.Member(41, "start-a", "/tmp/deps/abc-1234")
        self.assertIsNone(runner.start_clocks([first], clocks, 100.0))
        self.assertEqual(clocks, {(41, "start-a"): (104.0, "abc-1234")})
        self.assertIsNone(runner.start_clocks([first], clocks, 102.0))
        self.assertEqual(clocks, {(41, "start-a"): (104.0, "abc-1234")})
        execed = self.runner.Member(41, "start-a", "/usr/bin/other")
        self.assertIsNone(runner.start_clocks([execed], clocks, 103.0))
        self.assertEqual(clocks, {(41, "start-a"): (104.0, "abc-1234")})
        broken = self.runner.Member(41, "start-a", None, "unreadable exe")
        self.assertIsNone(runner.start_clocks([broken], clocks, 103.5))
        self.assertEqual(clocks, {(41, "start-a"): (104.0, "abc-1234")})
        reused = self.runner.Member(41, "start-b", "/tmp/deps/abc-1234")
        self.assertIsNone(runner.start_clocks([reused], clocks, 103.0))
        self.assertEqual(clocks, {(41, "start-b"): (107.0, "abc-1234")})
        self.assertIsNone(runner.start_clocks([], clocks, 104.0))
        self.assertEqual(clocks, {})

    def test_the_poll_interval_keeps_four_reads_per_bound(self):
        runner = self.runner.Runner(["unused"], 1, str(self.record), str(self.status))
        self.assertEqual(runner.poll_interval(), 0.25)
        runner.bound = 1800
        self.assertEqual(runner.poll_interval(), 1.0)

    def test_sigterm_is_forwarded_to_the_group_and_status_is_written(self):
        pidfile = self.root / "termed.pid"
        argv = self.deps_argv(
            "term", 'printf "%s\\n" "$$" > "$1"; sleep 600; :', pidfile)
        runner = subprocess.Popen(self.command(argv), stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE)
        try:
            pid = self.pid("termed.pid")
            runner.send_signal(signal.SIGTERM)
            stdout, stderr = runner.communicate(timeout=15)
            self.assertEqual(runner.returncode, 143, (stdout, stderr))
            self.assertEqual(self.status.read_text(), "exit 143\n")
            self.assertTrue(self.killed_after(pid))
        finally:
            if runner.poll() is None:
                runner.kill()
                runner.communicate()
            try:
                self.kill_group(self.pid("termed.pid"))
            except (ProcessLookupError, ValueError):
                pass

    def test_bad_utf8_output_is_passed_through_and_the_bound_still_fires(self):
        pidfile = self.root / "bad.pid"
        argv = self.deps_argv(
            "bad",
            'printf "\\377\\n"\n'
            'printf "%s\\n" "$$" > "$1"\n'
            "sleep 600; :\n", pidfile)
        result = self.launch(argv, bound=1)
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertIn(b"\xff\n", result.stdout)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "bad-0123abcd\n1\n")
        self.assertTrue(self.killed_after(self.pid("bad.pid")))

    def test_the_proc_source_skips_vanished_entries_and_refuses_an_unreadable_one(self):
        proc = self.root / "proc"
        (proc / "41").mkdir(parents=True)
        (proc / "41" / "stat").write_text(stat_line(41, 7, 12345))
        os.symlink(SHELL, proc / "41" / "exe")
        (proc / "42").mkdir()
        (proc / "42" / "stat").write_text(stat_line(42, 8, 999))
        os.symlink(SHELL, proc / "42" / "exe")
        self.assertEqual(self.runner.proc_group(7, root=proc),
                         ([self.runner.Member(41, 12345, SHELL)], None))
        (proc / "42" / "stat").unlink()
        (proc / "42" / "stat").mkdir()
        self.assertEqual(self.runner.proc_group(7, root=proc),
                         (None, "the process table could not be read"))
        (proc / "42" / "stat").rmdir()
        (proc / "42" / "stat").write_text("not a stat line\n")
        self.assertEqual(self.runner.proc_group(7, root=proc),
                         (None, "the process table printed a line that cannot be parsed"))
        shutil.rmtree(proc / "42")
        (proc / "41" / "exe").unlink()
        (proc / "41" / "exe").mkdir()
        (proc / "41" / "cmdline").mkdir()
        self.assertEqual(
            self.runner.proc_group(7, root=proc),
            ([self.runner.Member(
                41, 12345, None,
                "a member's executable could not be read, nor its arguments "
                "(EINVAL; stat state S, comm member)")],
             None))
        shutil.rmtree(proc)
        proc.mkdir()
        self.assertEqual(self.runner.proc_group(7, root=proc),
                         (None, "the process table read was empty"))

    def test_a_member_whose_exe_refuses_is_judged_by_its_first_argument(self):
        # #1054: the kernel can refuse a live member's exe link while its
        # arguments stay readable, so the member is judged by the path it
        # was started as; empty arguments still fail, and arguments that
        # are gone mean the member is gone.
        proc = self.root / "proc"
        (proc / "41").mkdir(parents=True)
        (proc / "41" / "stat").write_text(stat_line(41, 7, 12345))
        (proc / "41" / "exe").mkdir()
        dep = "/tmp/target/debug/deps/abc-1234"
        (proc / "41" / "cmdline").write_bytes(dep.encode() + b"\0--nocapture\0")
        why = "(EINVAL; stat state S, comm member)"
        judged = self.runner.Member(
            41, 12345, dep, None, f"its first argument, its executable unreadable {why}")
        self.assertEqual(self.runner.proc_group(7, root=proc), ([judged], None))
        self.assertEqual(self.runner.judged_name(judged), ("abc-1234", None))
        (proc / "41" / "cmdline").write_bytes(b"")
        self.assertEqual(
            self.runner.proc_group(7, root=proc),
            ([self.runner.Member(
                41, 12345, None,
                f"a member's executable could not be read, and its arguments are empty {why}")],
             None))
        # Empty arguments from a process that exited between the reads: gone.
        (proc / "41" / "stat").write_text(
            stat_line(41, 7, 12345).replace("(member) S", "(member) Z", 1))
        self.assertEqual(self.runner.exited(proc / "41"), True)
        self.assertEqual(self.runner.proc_group(7, root=proc), ([], None))
        (proc / "41" / "stat").write_text(stat_line(41, 7, 12345))
        (proc / "41" / "cmdline").unlink()
        self.assertEqual(self.runner.proc_group(7, root=proc), ([], None))

    def test_a_member_judged_by_its_argument_takes_a_clock_and_is_noted_once(self):
        runner = self.runner.Runner(["unused"], 4, str(self.record), str(self.status))
        clocks = {}
        judged = self.runner.Member(41, "start-a", "/tmp/deps/abc-1234", None,
                                    "its first argument, its executable unreadable (EACCES)")
        other = self.runner.Member(42, "start-b", "bwrap", None,
                                   "its first argument, its executable unreadable (EACCES)")
        noted = io.StringIO()
        with contextlib.redirect_stderr(noted):
            self.assertIsNone(runner.start_clocks([judged, other], clocks, 100.0))
            self.assertIsNone(runner.start_clocks([judged, other], clocks, 101.0))
        self.assertEqual(clocks, {(41, "start-a"): (104.0, "abc-1234")})
        self.assertEqual(noted.getvalue().splitlines(), [
            "shard runner: pid 41 is judged by its first argument, its executable "
            "unreadable (EACCES): /tmp/deps/abc-1234",
            "shard runner: pid 42 is judged by its first argument, its executable "
            "unreadable (EACCES): bwrap",
        ])

    def test_a_proc_entry_that_vanishes_mid_read_is_skipped(self):
        proc = self.root / "proc"
        for pid in ("41", "42"):
            (proc / pid).mkdir(parents=True)
            (proc / pid / "stat").write_text(stat_line(int(pid), 7, 12345))
            os.symlink(SHELL, proc / pid / "exe")
        original = Path.read_text

        def read_text(path, *args, **kwargs):
            if path.parent.name == "42":
                raise ProcessLookupError(3, "No such process")
            return original(path, *args, **kwargs)

        with mock.patch.object(Path, "read_text", new=read_text):
            self.assertEqual(self.runner.proc_group(7, root=proc),
                             ([self.runner.Member(41, 12345, SHELL)], None))

    def test_the_ps_source_parses_a_comm_with_spaces_and_refuses_a_bad_line(self):
        stub = self.root / "bin"
        stub.mkdir()
        ps = stub / "ps"
        ps.write_text(
            "#!/bin/sh\n"
            "printf '   41    41 Sun Nov  9 01:23:25 2025 "
            "/Applications/My App/rustdoc\\n'\n")
        ps.chmod(0o755)
        old = os.environ.get("PATH", "")
        os.environ["PATH"] = f"{stub}{os.pathsep}{old}"
        try:
            with mock.patch.object(self.runner.platform, "system", return_value="Darwin"):
                self.assertEqual(
                    self.runner.group_members(41),
                    ([self.runner.Member(41, "Sun Nov  9 01:23:25 2025",
                                         "/Applications/My App/rustdoc")], None))
                ps.write_text("#!/bin/sh\nprintf 'soon\\n'\n")
                self.assertEqual(self.runner.group_members(41),
                                 (None, "the process table printed a line that cannot be parsed"))
                ps.write_text("#!/bin/sh\nexit 1\n")
                self.assertEqual(self.runner.group_members(41),
                                 (None, "the process table could not be read"))
                ps.write_text("#!/bin/sh\nprintf '\\n'\n")
                self.assertEqual(self.runner.group_members(41),
                                 (None, "the process table read was empty"))
                ps.write_text("#!/bin/sh\nprintf '   42    43 Sun Nov  9 01:23:25 2025 x\\n'\n")
                self.assertEqual(self.runner.group_members(41), ([], None))
        finally:
            os.environ["PATH"] = old

    def test_a_binary_alive_after_argv_exits_is_still_bounded(self):
        pidfile = self.root / "leak.pid"
        dep = self.deps("leak")
        cargo = self.child(
            "\"$1\" -c 'sleep 3; :' leak-0123abcd &\n"
            "printf '%s\\n' \"$!\" > \"$2\"\n"
            "sleep 0.4\n"
            "printf 'test result: ok. 1 passed; 0 failed; 0 ignored; "
            "0 measured; 0 filtered out; finished in 0.40s\\n'\n"
            "exit 0\n", "leak-cargo")
        result = self.launch([cargo, dep, pidfile], bound=1)
        self.assertEqual(result.returncode, 124, result.stdout + result.stderr)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "leak-0123abcd\n1\n")
        self.assertTrue(self.killed_after(self.pid("leak.pid")))

    def test_a_binary_first_seen_after_argv_exits_is_still_bounded(self):
        # The binary first becomes a `deps` executable 0.5 s after cargo
        # exits. The settle window is widened to 10 s in this run, so its
        # first sight and its deadline both fall inside the window however
        # slowly the host starts it: at 2 s, a slow macOS start once
        # missed the deadline, and the binary passed.
        pidfile = self.root / "late.pid"
        dep = self.deps("late")
        cargo = self.child(
            "( sleep 0.5; exec \"$1\" -c 'sleep 3; :' late-0123abcd ) &\n"
            "printf '%s\\n' \"$!\" > \"$2\"\n"
            "sleep 0.2\n"
            "exit 0\n", "late-cargo")
        runner = self.runner.Runner([cargo, dep, pidfile], 1, str(self.record),
                                    str(self.status))
        with mock.patch.object(self.runner, "SETTLE_SECONDS", 10.0):
            code = runner.run()
        self.assertEqual(code, 124)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "late-0123abcd\n1\n")
        self.assertTrue(self.killed_after(self.pid("late.pid")))

    def test_a_deadline_read_happens_when_the_poll_interval_is_far_away(self):
        class LongPoll(self.runner.Runner):
            def poll_interval(self):
                return 60.0

        argv = self.deps_argv("sleepy", "sleep 4; :")
        runner = LongPoll(argv, 1, str(self.record), str(self.status))
        started = time.monotonic()
        code = runner.run()
        elapsed = time.monotonic() - started
        self.assertEqual(code, 124)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "sleepy-0123abcd\n1\n")
        self.assertGreaterEqual(elapsed, 1.0)
        self.assertLess(elapsed, 9.0)

    def test_a_read_at_the_deadline_sees_a_binary_the_poll_would_skip(self):
        # The poll is 60 s away and cargo outlives the binary, so only a read
        # at the clock's deadline can see it alive past its bound. The first
        # read waits until the binary has written its own pid, so its first
        # sight never rests on how fast the host starts it (macOS starts it
        # after a fast first read). Every read is delayed 0.06 s, as in the
        # review's case.
        class LongPoll(self.runner.Runner):
            def poll_interval(self):
                return 60.0

        pidfile = self.root / "pollgap.pid"
        dep = self.deps("pollgap")
        cargo = self.child(
            "\"$1\" -c 'printf \"%s\\n\" \"$$\" > \"$1\"; sleep 4; :' "
            "pollgap-0123abcd \"$2\" &\n"
            "wait\n"
            "sleep 0.6\n", "pollgap-cargo")
        original = self.runner.group_members

        def delayed(pgid):
            written = time.monotonic() + 5
            while not pidfile.exists() and time.monotonic() < written:
                time.sleep(0.01)
            time.sleep(0.06)
            return original(pgid)

        runner = LongPoll([cargo, dep, pidfile], 1, str(self.record),
                          str(self.status), read_group=delayed)
        code = runner.run()
        self.assertEqual(code, 124)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "pollgap-0123abcd\n1\n")
        self.assertTrue(self.killed_after(self.pid("pollgap.pid")))

    def test_a_signal_the_group_refuses_is_no_error_and_the_read_decides(self):
        # macOS answers EPERM, not ESRCH, when a group's last members are
        # zombies (#1027's first macOS run, where the forwarded SIGTERM's
        # second signal raised it). The refusal is no error: the group read
        # that follows still shows the group ended.
        runner = self.runner.Runner(["true"], 1, str(self.record), str(self.status))
        runner.process = subprocess.Popen(["true"], start_new_session=True)
        runner.pgid = runner.process.pid
        runner.eof = True
        refused = mock.Mock(side_effect=PermissionError(1, "Operation not permitted"))
        with mock.patch.object(self.runner.os, "killpg", refused):
            proven, reason = runner.end_group()
        self.assertEqual((proven, reason), (True, None))
        self.assertEqual(refused.call_args_list, [mock.call(runner.pgid, signal.SIGTERM)])

    def test_a_clock_starts_after_the_read_that_first_saw_it(self):
        pidfile = self.root / "early.pid"
        dep = self.deps("early")
        cargo = self.child(
            "sleep 0.6\n"
            "\"$1\" -c 'sleep 10; :' early-0123abcd &\n"
            "printf '%s\\n' \"$!\" > \"$2\"\n"
            "wait\n", "early-cargo")
        original = self.runner.group_members
        first = []

        def delayed(pgid):
            if not first:
                first.append(1)
                time.sleep(0.9)
            return original(pgid)

        runner = self.runner.Runner([cargo, dep, pidfile], 1, str(self.record),
                                    str(self.status), read_group=delayed)
        started = time.monotonic()
        code = runner.run()
        elapsed = time.monotonic() - started
        self.assertEqual(code, 124)
        self.assertEqual(self.status.read_text(), "bound\n")
        self.assertEqual(self.record.read_text(), "early-0123abcd\n1\n")
        self.assertGreaterEqual(
            elapsed, 1.8,
            "the bound fired before the first read plus the bound")

    def test_ps_args_reads_the_crate_name_under_darwin_and_refuses_unreadable_args(self):
        stub = self.root / "bin"
        stub.mkdir()
        ps = stub / "ps"
        ps.write_text(
            "#!/bin/sh\n"
            f"printf 'rustdoc {self.runner.DOCTEST_FLAG} --crate-name brokkr_core\\n'\n")
        ps.chmod(0o755)
        old = os.environ.get("PATH", "")
        os.environ["PATH"] = f"{stub}{os.pathsep}{old}"
        try:
            with mock.patch.object(self.runner.platform, "system", return_value="Darwin"):
                self.assertEqual(
                    self.runner.read_args(41),
                    ["rustdoc", self.runner.DOCTEST_FLAG, "--crate-name", "brokkr_core"])
                rustdoc = self.runner.Member(41, "stamp", "/usr/bin/rustdoc")
                self.assertEqual(
                    self.runner.test_binary_name(rustdoc, read=self.runner.read_args),
                    "Doc-tests brokkr_core")
                ps.write_text("#!/bin/sh\nexit 1\n")
                self.assertIsNone(self.runner.read_args(41))
                self.assertEqual(
                    self.runner.test_binary_name(rustdoc, read=self.runner.read_args),
                    "Doc-tests 41")
        finally:
            os.environ["PATH"] = old


if __name__ == "__main__":
    unittest.main()
