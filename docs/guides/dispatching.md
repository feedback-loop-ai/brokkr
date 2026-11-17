# Dispatching

`brokkr dispatch` starts the queue's entries as admission admits them,
and hands each run to a driver of its own (decision 0110). This guide
covers what it does on each tick, how to keep it running under a service
manager, and what each way it can end leaves behind, with how to recover
from it.

One dispatcher holds one journal. A second dispatcher on the same
journal is refused, and the refusal names the one that holds it.

## A tick

`brokkr dispatch --db <journal>` stands until it is stopped. Every
`--every <seconds>` (60 by default, from 5 to 3600) it runs one pass:

1. **Reconcile** what an earlier dispatcher left half-started. A claim
   with no run goes back to its place. A run linked with no `spawn` row is
   handed to a driver now.
2. **Sweep** the seat trees that closed effects left behind.
3. **Read the bound**: the number of counted refusals after which an
   entry is held.
4. **Judge the queue**, as `brokkr queue judge` would.
5. **Start each admitted entry**, in the queue's order. Each entry gets
   its claim, its run, its link, its `spawn` row and its driver before the
   next entry is claimed.

The pass then prints what it did, one line per fact, the same lines
`brokkr dispatch --once` prints. After those come one line for each
driver that has exited since the last tick, with its run, its pid and its
exit status. The next pass starts `--every` seconds after this one began.
A pass that took longer than that is followed by the next at once.

A driver is `brokkr dispatch drive`, run from the dispatcher's own binary
in a session of its own. Its output goes to `<journal>.runs/<run>.log`,
beside the journal. Brokkr never removes these logs.

`brokkr dispatch --once` runs a single pass and returns without waiting
on the drivers it started. `--every` cannot be given with `--once`.

What a pass meets is reported, and the next pass judges it again. That
covers an unreadable host file, an entry that does not read, a contended
write and a refused claim: nothing latches. The dispatcher never resumes,
adopts, stops or concludes a run.

## Stopping it

The first SIGINT or SIGTERM:

- wakes the dispatcher at once if it is waiting for its next tick;
- lets an entry already in flight get its `spawn` row and its driver;
- claims nothing after that.

The dispatcher then prints that tick's lines, names each driver still
running with `left running:`, and exits 0 with a `stopped:` line.

A second signal while the first is being handled ends the process at
once, with exit 128 plus the signal's number: 130 for SIGINT and 143 for
SIGTERM. Treat that as a crash (see below).

The drivers are never signalled. Each one leads its own session, so a
signal sent to the dispatcher, or to its process group, does not reach
them. They run on and finish their runs after the dispatcher has gone.

## Under a systemd user unit

Brokkr installs no service file. A user unit like this one runs the
dispatcher for one journal:

```ini
[Unit]
Description=Brokkr dispatcher for ~/work's journal

[Service]
WorkingDirectory=%h/work
Environment=PATH=%h/.local/bin:/usr/local/bin:/usr/bin:/bin
ExecStart=%h/.local/bin/brokkr dispatch --every 60 --db %h/work/.forge/forge.db
KillMode=process

[Install]
WantedBy=default.target
```

- **`KillMode=process` is required.** Drivers stay in the unit's cgroup.
  Under the default `KillMode=control-group`, stopping the unit would kill
  every driver with it. With `KillMode=process`, `systemctl --user stop`
  sends SIGTERM to the dispatcher alone.
- **`PATH`** is what each driver's seats will find. Name the directories
  that hold the provider CLIs your recipes seat.
- **No `Restart=`.** Each ending below is meant for you to read before
  the dispatcher starts again. Restarting it on its own would hide that.
- When the unit starts again while drivers from its last run are still
  going, systemd warns about left-over processes. That is expected.

Install it as `~/.config/systemd/user/brokkr-dispatch.service`, then:

```sh
systemctl --user daemon-reload
systemctl --user enable --now brokkr-dispatch.service
journalctl --user -u brokkr-dispatch.service -f
```

## Under a launchd agent

On macOS, a launch agent does the same job:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>local.brokkr.dispatch</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/brokkr</string>
    <string>dispatch</string>
    <string>--every</string><string>60</string>
    <string>--db</string><string>/Users/you/work/.forge/forge.db</string>
  </array>
  <key>WorkingDirectory</key><string>/Users/you/work</string>
  <key>EnvironmentVariables</key>
  <dict><key>PATH</key><string>/usr/local/bin:/usr/bin:/bin</string></dict>
  <key>StandardOutPath</key><string>/Users/you/Library/Logs/brokkr-dispatch.log</string>
  <key>StandardErrorPath</key><string>/Users/you/Library/Logs/brokkr-dispatch.log</string>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><false/>
  <key>AbandonProcessGroup</key><true/>
</dict>
</plist>
```

- **`AbandonProcessGroup`** stops launchd from killing the job's process
  group when the dispatcher exits. The drivers lead their own sessions
  anyway, so they are outside that group. The key makes that explicit.
- **`KeepAlive` is false**, for the same reason the systemd unit sets no
  `Restart=`.

Save it as `~/Library/LaunchAgents/local.brokkr.dispatch.plist`, then:

```sh
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/local.brokkr.dispatch.plist
launchctl kill SIGTERM gui/$(id -u)/local.brokkr.dispatch
launchctl bootout gui/$(id -u)/local.brokkr.dispatch
```

`launchctl kill SIGTERM` stops the dispatcher and leaves the agent
loaded. `bootout` unloads it, and also stops it with SIGTERM.

## What an ending leaves, and how to recover

Apart from a stop, the dispatcher ends with a line that begins `ended:`,
and exits non-zero. Read that line, then act on it as below.

| Ending | What it leaves | How to recover |
|---|---|---|
| **A crash** (killed, a second signal, the host went down) | Possibly one entry `launching`, or linked to a run with no `spawn` row. Drivers already started run on. | Start the dispatcher again. Its first pass reconciles: a claim with no run goes back to its place uncounted, and a linked run is handed to a driver. A run that already has its `spawn` row is never spawned twice. |
| **Fenced out** (`generation N was fenced out`) | Nothing written after the fence. A newer dispatcher holds the journal. | Run `brokkr dispatch status --db <journal>` to see who holds it. Start a dispatcher only if it says the journal is not held. |
| **A replaced binary** (`the brokkr at … is not the one this dispatcher started from`) | Nothing claimed after the check. An entry already linked waits. | Start the dispatcher again, from the new binary. Its reconciliation hands the waiting entry to a driver of the new version. |
| **Any other start refusal**, or a store error | The entry in flight stays `launching`. The line names the refusal. | Fix what the line names (the journal or the host), then start the dispatcher again. Reconciliation returns the entry to its place uncounted. |
| **Queue storage this binary does not write** | Nothing written. | Run a brokkr that writes the journal's queue storage version. |

Two outcomes do not end the dispatcher, but leave a run for you to deal
with:

- **A driver that died.** The tick's line reads `driver exited: run …,
  driver pid …, …; the run reads running with no driver; resume it with:
  brokkr resume --run=<run> --db=<journal>`. The run is never spawned again,
  and the dispatcher does not adopt it. Read the run's log beside the
  journal, then paste the command.
- **A spawn failure.** The line reads `spawn failed: run … reads running
  with no driver (…); resume it with: brokkr resume --run=<run>
  --db=<journal>`. The run's `spawn` row already stands, so reconciliation
  will not spawn it again. Paste the command once the cause (for example,
  no free descriptor) has cleared.

A driver that exits once its run has finished, with the run reading
`completed` or anything else that is not `running`, needs nothing from
you.

## The bound and the readout

- `brokkr dispatch set-bound <n> --reason <text> --db <journal>` sets how
  many counted start refusals hold an entry. The value is a whole number of
  at least 1, and 3 unless set. It is journaled as your command, and the
  next pass reads it.
- `brokkr dispatch status --db <journal>` opens the journal read-only and
  writes nothing. It says one of these:
  - `held by generation N (pid P on this host since T)`;
  - `not held; last held by generation N …`;
  - `not held; no dispatcher has taken this journal`.
  It also names the bound in force, and each entry left `launching`, or
  linked with no `spawn` row.

`brokkr queue list` is where you read each entry's verdict.
