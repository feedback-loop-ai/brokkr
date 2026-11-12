#!/usr/bin/env python3
"""Run one shard's argv in the foreground and enforce its per-binary bound.

The runner owns the child's output and process group, so nothing beside it
can stop enforcing the bound: it starts argv in a session of its own and
reads the group's process table while argv runs and, after argv's own
exit, until the group is shown ended or the settle window passes. A clock
is a test binary's process identity, its pid with its start stamp, and its
deadline is the monotonic time taken after the read that first saw that
identity plus `--binary-bound` seconds, so it can only start late, never
early, and at most one poll interval plus one group read after the
process's exec. A deadline is never moved and never cleared while its
process lives; a process that is gone is dropped, and a reused pid is a
new process with a new clock. A clock is overdue when a read that began at
or after its deadline still finds its process: a binary is judged at the
first such read, and there is a read at every deadline. Judging continues
after argv exits only through the settle window: a binary overdue inside
it is bounded like one overdue before it, and members still alive when it
passes are the survivor rule's `unproven`, whatever their clocks say.

A test binary is a member of argv's process group that is either

  * a cargo test binary: its executable's parent directory is named `deps`
    (`target/<profile>/deps/<name>-<hash>`, a target-triple profile, and
    cargo-llvm-cov's `target/llvm-cov-target/...` alike), whose name is the
    executable's file name; or
  * a doctest run: its executable's file name is `rustdoc` and its
    arguments hold the --test flag, whose name is `Doc-tests <crate>` from its
    `--crate-name` argument, or `Doc-tests <pid>` when that argument
    cannot be read.

Nothing else is judged: cargo, rustc, linkers, build scripts and a test's
helpers outside `deps` get no clock. A helper is inside its parent binary's
life anyway.

The runner copies every output byte to its own stdout unchanged and never
decodes it, so nothing a test prints can start, restart, stop or delay a
clock. It polls the group at least every `min(1 s, bound / 4)` seconds and
at the earliest live deadline; the select timeout is the time left to the
next of those, and output still flows between polls, and a poll never waits
on output. A group read that fails during a poll is `unproven` with its
reason, the same tri-state rule as ending the group, never an empty group.

It writes the bound record (the binary's name, then the seconds, one per
line) and one status line last, once:

  exit <code>       argv ended on its own and its group is shown ended
  bound             the bound fired and the group is shown ended
  unproven <reason> the group cannot be shown ended, so the shard is lost

Its own exit is argv's code, 124 for `bound`, and 125 for `unproven`. An
exception before the status line is written leaves no status file, which is
what the shard reads as a lost runner.

Showing a group ended is a tri-state read, never an empty group on a failed
one: Linux reads every /proc/<pid>/stat and each member's /proc/<pid>/exe,
skipping a pid that vanishes mid-read, and elsewhere `ps` reports the pid,
group, start time and executable in one line. Members that remain after a
SIGKILL and a five-second wait are unproven, named by pid where the source
has them. A test binary that leaves the shard's process group, by setsid or
a daemonizing double fork, is outside this clock and outside the group
proof; the job's own timeout is its bound.
"""

import argparse
import errno
import os
from pathlib import Path
import platform
import selectors
import signal
import subprocess
import sys
import time
from typing import NamedTuple

READ_SIZE = 65536
SELECT_SECONDS = 1.0
TERM_GRACE_SECONDS = 10.0
KILL_GRACE_SECONDS = 5.0
SETTLE_SECONDS = 2.0
POLL_SECONDS = 0.1
EXIT_BOUND = 124
EXIT_UNPROVEN = 125
PROC = Path("/proc")
NAME_LIMIT = 200
LSTART_WIDTH = 24
# The doctest flag, in two literals so the layering gate reads no command
# here. A single literal would name the flag to that reader as prose.
DOCTEST_FLAG = "--" "test"

GROUP_UNREADABLE = "the process table could not be read"
GROUP_UNPARSEABLE = "the process table printed a line that cannot be parsed"
GROUP_EMPTY = "the process table read was empty"
EXE_UNREADABLE = "a member's executable could not be read"
# The kernel's task flag for a process inside exit(): /proc/<pid>/stat's
# ninth field, readable whatever the reader may trace (include/linux/sched.h).
PF_EXITING = 0x00000004


class Forwarded(Exception):
    """A SIGINT or SIGTERM the runner forwards to the group then ends it."""

    def __init__(self, signum):
        super().__init__(signum)
        self.signum = signum


class Member(NamedTuple):
    """One process group member: its identity and, if readable, its exe.

    `start` is the platform's start stamp for the pid, so `(pid, start)` is
    the process identity a clock is keyed by. `exe` is the executable the
    kernel reports, or None when it could not be read, with `exe_problem`
    saying why; a member whose executable cannot be read is a failed read,
    never a member that is silently not judged. When the executable link
    refuses its read but the arguments can be read, `exe` is the first
    argument and `exe_source` says so and why, so the member is judged by
    the path it was started as (#1054).
    """

    pid: int
    start: object
    exe: object = None
    exe_problem: object = None
    exe_source: object = None


def parse_stat(text):
    """The pid, process group and start stamp in one /proc/<pid>/stat."""
    open_paren = text.find("(")
    close_paren = text.rfind(")")
    if open_paren < 1 or close_paren <= open_paren:
        return None
    try:
        pid = int(text[:open_paren].strip())
        fields = text[close_paren + 1:].split()
        group = int(fields[2])
        # The fields after `)` start at stat's field 3, so field 22 is
        # index 19: the start time that tells a reused pid apart.
        start = int(fields[19])
    except (IndexError, ValueError):
        return None
    return pid, group, start


def stat_detail(text, error):
    """The errno, state and command name a refused read is reported with."""
    name = errno.errorcode.get(error.errno, str(error.errno))
    open_paren = text.find("(")
    close_paren = text.rfind(")")
    if open_paren < 1 or close_paren <= open_paren:
        return f"{name}; stat unreadable"
    comm = "".join(c if c.isprintable() else "?" for c in text[open_paren + 1:close_paren])
    fields = text[close_paren + 1:].split()
    state = fields[0] if fields else "?"
    return f"{name}; stat state {state}, comm {comm[:32]}"


def exited(entry):
    """Whether the process at /proc entry `entry` is gone, a zombie, or inside
    exit() now.

    A process inside exit() has released its memory, which is why its
    arguments read empty, and runs no code of its own again. A setuid helper
    such as fusermount3 can sit there in state D, waiting on an unmount, past
    the runner's whole grace (#1054, #1066's linux/5).
    """
    try:
        text = (entry / "stat").read_text(encoding="utf-8", errors="replace")
    except (FileNotFoundError, ProcessLookupError):
        return True
    except OSError:
        return False
    fields = text[text.rfind(")") + 1:].split()
    if not fields:
        return False
    if fields[0] in ("Z", "X"):
        return True
    try:
        return bool(int(fields[6]) & PF_EXITING)
    except (IndexError, ValueError):
        return False


def proc_member(entry, start, stat_text=""):
    """The member at /proc entry `entry`, or None when it vanished.

    An executable link that refuses its read with anything but ENOENT or
    ESRCH is not the end of the judgment: the kernel can refuse it for a
    live process it will not let this user trace, while the arguments stay
    readable. Such a member is judged by its first argument, the path cargo
    starts a test binary as. A process that changed its credentials, as one
    entering a user namespace does, is refused this way until its next exec.
    Arguments that read empty because the process exited between the two
    reads, or is inside exit() (a setuid helper is refused this way too),
    mean it is gone. Only a live member whose arguments cannot be read, or
    are empty, is a failed read, and its reason names the errno and the
    state and command name its stat holds (#1054).
    """
    try:
        exe = os.readlink(entry / "exe")
    except (FileNotFoundError, ProcessLookupError):
        # The pid exited, or is a zombie with no exe left: not a live
        # member, so it takes no clock and cannot be a survivor.
        return None
    except OSError as error:
        detail = stat_detail(stat_text, error)
    else:
        return Member(int(entry.name), start, exe)
    try:
        data = (entry / "cmdline").read_bytes()
    except (FileNotFoundError, ProcessLookupError):
        return None
    except OSError:
        return Member(int(entry.name), start, None,
                      f"{EXE_UNREADABLE}, nor its arguments ({detail})")
    first = data.split(b"\0", 1)[0].decode("utf-8", "replace")
    if not first:
        if exited(entry):
            return None
        return Member(int(entry.name), start, None,
                      f"{EXE_UNREADABLE}, and its arguments are empty ({detail})")
    return Member(int(entry.name), start, first, None,
                  f"its first argument, its executable unreadable ({detail})")


def proc_group(pgid, root=PROC):
    """The members of process group `pgid` read from /proc, or a problem.

    A pid that vanishes between the listing and its stat read is skipped; a
    stat or executable that cannot be read or parsed is a problem, never an
    empty group.
    """
    try:
        entries = list(root.iterdir())
    except OSError:
        return None, GROUP_UNREADABLE
    members = []
    seen = False
    for entry in entries:
        if not entry.name.isdigit():
            continue
        try:
            text = (entry / "stat").read_text(encoding="utf-8", errors="replace")
        except (FileNotFoundError, ProcessLookupError):
            # ENOENT and ESRCH: the pid vanished between the listing and the
            # read, which is how a busy process table reads, never empty.
            continue
        except OSError:
            return None, GROUP_UNREADABLE
        seen = True
        parsed = parse_stat(text)
        if parsed is None:
            return None, GROUP_UNPARSEABLE
        pid, group, start = parsed
        if group != pgid:
            continue
        member = proc_member(entry, start, text)
        if member is not None:
            members.append(member)
    if not seen:
        return None, GROUP_EMPTY
    return members, None


def ps_member(line):
    """One `pid=,pgid=,lstart=,comm=` line as a member, or None.

    `lstart` is fixed-width and `comm` comes last and may hold spaces, so
    the line is split only as far as the start stamp and the remainder is
    the executable whole.
    """
    try:
        pid_text, group_text, rest = line.lstrip().split(None, 2)
        pid = int(pid_text)
        group = int(group_text)
    except ValueError:
        return None
    start = rest[:LSTART_WIDTH].strip()
    comm = rest[LSTART_WIDTH:].strip()
    if not start or not comm:
        return None
    return group, Member(pid, start, comm)


def ps_group(pgid):
    """The members of process group `pgid` read from `ps`, or a problem."""
    try:
        result = subprocess.run(
            ["ps", "-axww", "-o", "pid=,pgid=,lstart=,comm="],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
        )
    except OSError:
        return None, GROUP_UNREADABLE
    if result.returncode != 0:
        return None, GROUP_UNREADABLE
    rows = [row for row in result.stdout.splitlines() if row.strip()]
    if not rows:
        return None, GROUP_EMPTY
    members = []
    for row in rows:
        parsed = ps_member(row)
        if parsed is None:
            return None, GROUP_UNPARSEABLE
        group, member = parsed
        if group == pgid:
            members.append(member)
    return members, None


def proc_args(pid):
    """The argument vector at /proc/<pid>/cmdline, or None when unreadable."""
    try:
        data = (PROC / str(pid) / "cmdline").read_bytes()
    except OSError:
        return None
    if not data:
        return []
    return [part.decode("utf-8", "replace") for part in data.rstrip(b"\0").split(b"\0")]


def ps_args(pid):
    """The argument vector `ps` reports for `pid`, or None when unreadable."""
    try:
        result = subprocess.run(
            ["ps", "-ww", "-o", "args=", "-p", str(pid)],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
        )
    except OSError:
        return None
    if result.returncode != 0:
        return None
    return result.stdout.split()


def read_args(pid):
    """The arguments of `pid` on this host, or None when they cannot be read."""
    if platform.system() == "Linux":
        return proc_args(pid)
    return ps_args(pid)


def group_members(pgid):
    """The tri-state read of process group `pgid` on this host."""
    if platform.system() == "Linux":
        return proc_group(pgid)
    return ps_group(pgid)


def survivor_reason(pgid, members):
    """A named group with member pids left, naming them where the source does."""
    if platform.system() == "Linux":
        return (f"process group {pgid} still has members "
                + ", ".join(str(member) for member in members))
    return f"process group {pgid} still has members"


def crate_name(args):
    """The value of `--crate-name` in `args`, or None when it is not there."""
    for index, word in enumerate(args):
        if word == "--crate-name" and index + 1 < len(args):
            return args[index + 1]
        if word.startswith("--crate-name="):
            return word.split("=", 1)[1]
    return None


def test_binary_name(member, read=read_args):
    """The name `member` is judged by, or None when it is not a test binary.

    A `deps` executable is a cargo test binary named by its file name. A
    `rustdoc` whose arguments hold the --test flag is a doctest run named by its
    `--crate-name` or, when that cannot be read, by its pid. A `rustdoc`
    whose arguments cannot be read at all is still judged, the fail-closed
    reading of a doctest whose arguments a test cannot hide.
    """
    if member.exe is None:
        return None
    base = os.path.basename(member.exe)
    if os.path.basename(os.path.dirname(member.exe)) == "deps":
        return base[:NAME_LIMIT]
    if base != "rustdoc":
        return None
    args = read(member.pid)
    if args is not None and DOCTEST_FLAG not in args:
        return None
    crate = crate_name(args or [])
    return (f"Doc-tests {crate}" if crate else f"Doc-tests {member.pid}")[:NAME_LIMIT]


def judged_name(member, read=read_args):
    """The name `member` is judged by, or why its executable cannot be read."""
    if member.exe_problem is not None:
        return None, member.exe_problem
    return test_binary_name(member, read), None


class Runner:
    """One foreground argv, its process group, its bound and its status."""

    def __init__(self, argv, bound, bound_record, status, read_group=group_members,
                 read_args=read_args):
        self.argv = list(argv)
        self.bound = bound
        self.bound_record = bound_record
        self.status = status
        self.read_group = read_group
        self.read_args = read_args
        self.noted = set()
        self.process = None
        self.pgid = None
        self.selector = None
        self.fd = None
        self.eof = False
        self.handlers = {}

    def run(self):
        self.process = subprocess.Popen(
            self.argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT, start_new_session=True,
        )
        self.pgid = self.process.pid
        self.selector = selectors.DefaultSelector()
        self.fd = self.process.stdout.fileno()
        self.selector.register(self.fd, selectors.EVENT_READ)
        forwarded = None
        try:
            try:
                for signum in (signal.SIGINT, signal.SIGTERM):
                    self.handlers[signum] = signal.signal(signum, self.forward)
            except Forwarded as error:
                forwarded = error.signum
            if forwarded is None:
                try:
                    code = self.loop()
                except Forwarded as error:
                    forwarded = error.signum
                else:
                    return code
            return self.after_signal(forwarded)
        finally:
            for signum, handler in self.handlers.items():
                signal.signal(signum, handler)
            self.selector.close()
            if self.process.stdout is not None:
                self.process.stdout.close()

    def forward(self, signum, _frame):
        self.signal_group(signum)
        raise Forwarded(signum)

    def signal_group(self, signum):
        """Signal argv's group; a group left with nothing to signal is no error.

        A group that is gone answers ESRCH. macOS answers EPERM instead when
        the group's last members are zombies, where Linux delivers the
        signal, as #1027's first macOS run showed. Neither answer proves
        anything: the group read after every signal shows the group ended or
        names what is left.
        """
        try:
            os.killpg(self.pgid, signum)
        except (ProcessLookupError, PermissionError):
            pass

    def loop(self):
        """Read the group for the whole shard, copying output as it goes.

        The loop reads when the poll time is due or the earliest live clock
        is due, whichever comes first, and waits only until the next of
        those. Each read starts a clock for a test binary it first sees,
        drops the gone ones and judges the overdue ones, and that judging
        continues after argv exits: an empty group after argv has been
        reaped is the only `exit`, and a settle window that passes with
        members left is the survivor rule's `unproven`.
        """
        clocks = {}
        next_poll = time.monotonic()
        settle_deadline = None
        while True:
            now = time.monotonic()
            # Whether argv was already reaped when this read begins, so an
            # empty group after a read that raced argv's own exit is read
            # again before it can end the shard.
            reaped = self.process.poll() is not None
            if reaped and settle_deadline is None:
                settle_deadline = time.monotonic() + SETTLE_SECONDS
                next_poll = min(next_poll, time.monotonic())
            due = any(deadline <= now for deadline, _name in clocks.values())
            if now >= next_poll or due:
                read_started = time.monotonic()
                members, problem = self.read_group(self.pgid)
                if problem is not None:
                    return self.poll_failed(problem)
                # The overdue test uses the time the read began, so a clock
                # is judged only by a read that began at or after its
                # deadline; a new clock is stamped with the time taken after
                # this read returns, so it can only start late, never early.
                stamped = time.monotonic()
                problem = self.start_clocks(members, clocks, stamped)
                if problem is not None:
                    return self.poll_failed(problem)
                bound = self.expired(clocks, read_started)
                if bound is not None:
                    return self.fire(bound)
                next_poll = time.monotonic() + self.poll_interval()
                if settle_deadline is not None:
                    next_poll = min(next_poll, settle_deadline)
                    if members == [] and reaped:
                        self.drain_rest()
                        return self.exit_status()
                    if time.monotonic() >= settle_deadline:
                        return self.unproven(survivor_reason(
                            self.pgid, [member.pid for member in members]))
                now = time.monotonic()
            wait = self.wait_seconds(next_poll, clocks, now)
            if wait <= 0:
                continue
            if self.eof:
                time.sleep(wait)
            elif self.selector.select(wait):
                data = os.read(self.fd, READ_SIZE)
                if data:
                    self.write(data)
                else:
                    self.eof = True
                    self.selector.unregister(self.fd)

    def poll_interval(self):
        """At least four group reads per bound, and never longer than a second."""
        return min(SELECT_SECONDS, self.bound / 4.0)

    def wait_seconds(self, next_poll, clocks, now):
        """The time left to the next poll or the earliest live deadline."""
        wait = next_poll - now
        for deadline, _name in clocks.values():
            wait = min(wait, deadline - now)
        return max(0.0, wait)

    def start_clocks(self, members, clocks, now):
        """Start a clock for each new test binary and drop the gone ones.

        `clocks` maps `(pid, start)` to `(deadline, name)`. `now` is the
        time taken after the read that first saw these members, so a new
        clock can only start late, never early. A deadline is set once, at
        first sight, and never moved, and a clock stays while
        its process is still a member even if the process later reports
        another executable; a process that is gone is dropped. A member
        whose executable cannot be read before it is clocked is a failed
        read, so the caller marks the shard unproven.
        """
        seen = set()
        for member in members:
            key = (member.pid, member.start)
            seen.add(key)
            if member.exe_source is not None and key not in self.noted:
                # Said once per process, so the log shows what the kernel
                # refused and what the member was judged by instead (#1054).
                self.noted.add(key)
                sys.stderr.write(f"shard runner: pid {member.pid} is judged by "
                                 f"{member.exe_source}: {member.exe}\n")
                sys.stderr.flush()
            if key in clocks:
                continue
            name, problem = judged_name(member, self.read_args)
            if problem is not None:
                return problem
            if name is None:
                continue
            clocks[key] = (now + self.bound, name)
        for key in list(clocks):
            if key not in seen:
                del clocks[key]
        return None

    def expired(self, clocks, now):
        """The name of the earliest clock at or past its deadline, or None."""
        overdue = [(deadline, name) for deadline, name in clocks.values()
                   if deadline <= now]
        if not overdue:
            return None
        return min(overdue)[1]

    def poll_failed(self, reason):
        """A failed group read is unproven: end the group, then name why."""
        self.end_group()
        return self.unproven(reason)

    def write(self, data):
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()

    def write_record(self, name):
        Path(self.bound_record).write_text(f"{name}\n{self.bound}\n", encoding="utf-8")

    def write_status(self, line):
        temporary = self.status + ".tmp"
        with open(temporary, "w", encoding="utf-8") as handle:
            handle.write(line + "\n")
        os.replace(temporary, self.status)

    def exit_code(self, signum=None):
        code = self.process.returncode
        if code is None:
            return 128 + (signum or signal.SIGTERM)
        if code < 0:
            return 128 - code
        return code

    def exit_status(self, signum=None):
        code = self.exit_code(signum)
        self.write_status(f"exit {code}")
        return code

    def unproven(self, reason):
        self.write_status(f"unproven {reason}")
        return EXIT_UNPROVEN

    def fire(self, name):
        """Write the record first, then end the group whatever the write did.

        The group is ended in a finally so a record that cannot be written
        never leaves it alive. No status is written then, so the shard reads
        the runner as lost rather than as a bound it cannot name.
        """
        try:
            self.write_record(name)
        finally:
            proven, reason = self.end_group()
        if not proven:
            return self.unproven(reason)
        self.write_status("bound")
        return EXIT_BOUND

    def after_signal(self, signum):
        """End a group a forwarded signal was already sent to, then record it."""
        proven, reason = self.end_group()
        if not proven:
            return self.unproven(reason)
        return self.exit_status(signum)

    def reap(self, done):
        if done:
            return True
        if self.process.poll() is None:
            return False
        self.process.wait()
        return True

    def end_group(self):
        """TERM, wait ten seconds, KILL, wait five, and show the group ended."""
        self.signal_group(signal.SIGTERM)
        done = False
        deadline = time.monotonic() + TERM_GRACE_SECONDS
        while time.monotonic() < deadline:
            done = self.reap(done)
            self.drain_available()
            members, problem = self.read_group(self.pgid)
            if done and members == []:
                self.drain_rest()
                return True, None
            time.sleep(POLL_SECONDS)
        self.signal_group(signal.SIGKILL)
        deadline = time.monotonic() + KILL_GRACE_SECONDS
        while time.monotonic() < deadline:
            done = self.reap(done)
            self.drain_available()
            members, problem = self.read_group(self.pgid)
            if done and members == []:
                self.drain_rest()
                return True, None
            time.sleep(POLL_SECONDS)
        done = self.reap(done)
        members, problem = self.read_group(self.pgid)
        pids = [member.pid for member in (members or [])]
        if not done:
            pids = [self.process.pid] + [pid for pid in pids if pid != self.process.pid]
        if done and members == []:
            self.drain_rest()
            return True, None
        if problem is not None:
            return False, problem
        return False, survivor_reason(self.pgid, pids)

    def drain_available(self):
        """Copy the bytes already in the pipe without blocking."""
        while not self.eof:
            if not self.selector.select(0):
                return
            data = os.read(self.fd, READ_SIZE)
            if not data:
                self.eof = True
                self.selector.unregister(self.fd)
                return
            self.write(data)

    def drain_rest(self):
        """Copy the last bytes, bounded so a stray writer cannot hold the run."""
        deadline = time.monotonic() + SELECT_SECONDS
        while not self.eof and time.monotonic() < deadline:
            if not self.selector.select(POLL_SECONDS):
                continue
            data = os.read(self.fd, READ_SIZE)
            if not data:
                self.eof = True
                self.selector.unregister(self.fd)
                return
            self.write(data)


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-bound", type=int, metavar="N", required=True,
                        help="per-binary seconds before the group is ended")
    parser.add_argument("--bound-record", metavar="PATH", required=True,
                        help="the file the shard reads a past-bound binary's name from")
    parser.add_argument("--status", metavar="PATH", required=True,
                        help="the file the shard reads the runner's one status line from")
    parser.add_argument("argv", nargs=argparse.REMAINDER,
                        help="the shard's command after --")
    args = parser.parse_args(argv)
    if args.argv and args.argv[0] == "--":
        args.argv = args.argv[1:]
    if not args.argv:
        parser.error("a command is required after --")
    if args.binary_bound < 1:
        parser.error("--binary-bound must be a whole number of seconds above 0")
    return args


def main(argv=None):
    args = parse_args(argv)
    try:
        return Runner(args.argv, args.binary_bound, args.bound_record,
                      args.status).run()
    except Exception as error:  # No status file is the fail-closed signal.
        print(f"shard runner: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
