//! The three raw `launchctl print` samples the fa7ece5 native run (CI
//! `34457208029`) actually recorded, kept verbatim as the probe's
//! regression test data. They are test data only; nothing here runs. The
//! form is what macOS printed: tabs, nested blocks, an absent `successive
//! crashes` counter and the literal `(never exited)`.

pub const FA7_S2_RUNNING: &str = r##"gui/501/org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed = {
	active count = 1
	path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed/startup.plist
	type = LaunchAgent
	state = running

	program = /Users/runner/work/brokkr/brokkr/target/debug/seatbelt-probe-helper
	arguments = {
		/Users/runner/work/brokkr/brokkr/target/debug/seatbelt-probe-helper
		startup
		--root
		/private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed
		--nonce
		48523-1789030276013157000
		--exit
		clean
	}

	stdout path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed/startup.out
	stderr path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed/startup.err
	inherited environment = {
		SSH_AUTH_SOCK => /var/run/com.apple.launchd.CFkKnLTePe/Listeners
	}

	default environment = {
		PATH => /usr/bin:/bin:/usr/sbin:/sbin
	}

	environment = {
		OSLogRateLimit => 64
		XPC_SERVICE_NAME => org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed
	}

	domain = gui/501 [100002]
	asid = 100002
	minimum runtime = 10
	exit timeout = 5
	runs = 1
	pid = 48630
	immediate reason = speculative
	forks = 1
	execs = 1
	initialized = 1
	trampolined = 1
	started suspended = 0
	proxy started suspended = 0
	checked allocations = 0 (queried = 1)
	checked allocations reason = no host
	checked allocations flags = 0x0
	last exit code = (never exited)

	resource coalition = {
		ID = 1231
		type = resource
		state = active
		active count = 1
		name = org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed
	}

	jetsam coalition = {
		ID = 1232
		type = jetsam
		state = active
		active count = 1
		name = org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed
	}

	spawn type = daemon (3)
	jetsam priority = 40
	jetsam memory limit (active) = (unlimited)
	jetsam memory limit (inactive) = (unlimited)
	jetsamproperties category = daemon
	jetsam thread limit = 32
	cpumon = default

	properties = runatload | inferred program | system service | tle system
}"##;

pub const FA7_S2_NOT_RUNNING: &str = r##"gui/501/org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed = {
	active count = 0
	path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed/startup.plist
	type = LaunchAgent
	state = not running

	program = /Users/runner/work/brokkr/brokkr/target/debug/seatbelt-probe-helper
	arguments = {
		/Users/runner/work/brokkr/brokkr/target/debug/seatbelt-probe-helper
		startup
		--root
		/private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed
		--nonce
		48523-1789030276013157000
		--exit
		clean
	}

	stdout path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed/startup.out
	stderr path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S2.launchd.unboxed/startup.err
	inherited environment = {
		SSH_AUTH_SOCK => /var/run/com.apple.launchd.CFkKnLTePe/Listeners
	}

	default environment = {
		PATH => /usr/bin:/bin:/usr/sbin:/sbin
	}

	environment = {
		OSLogRateLimit => 64
		XPC_SERVICE_NAME => org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed
	}

	domain = gui/501 [100002]
	asid = 100002
	minimum runtime = 10
	exit timeout = 5
	runs = 1
	last exit code = 0

	resource coalition = {
		ID = 1231
		type = resource
		state = active
		active count = 1
		name = org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed
	}

	jetsam coalition = {
		ID = 1232
		type = jetsam
		state = active
		active count = 1
		name = org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.1.S2.launchd.unboxed
	}

	spawn type = daemon (3)
	jetsam priority = 40
	jetsam memory limit (active) = (unlimited)
	jetsam memory limit (inactive) = (unlimited)
	jetsamproperties category = daemon
	jetsam thread limit = 32
	cpumon = default

	properties = runatload | inferred program | system service | tle system
}"##;

pub const FA7_S3_RUNNING: &str = r##"gui/501/org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.2.S3.launchd.seatbelt = {
	active count = 0
	path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S3.launchd.seatbelt/startup.plist
	type = LaunchAgent
	state = not running

	program = /usr/bin/sandbox-exec
	arguments = {
		/usr/bin/sandbox-exec
		-f
		/private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S3.launchd.seatbelt/inputs/policy.sb
		/Users/runner/work/brokkr/brokkr/target/debug/seatbelt-probe-helper
		startup
		--root
		/private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S3.launchd.seatbelt
		--nonce
		48523-1789030276013157000
		--exit
		clean
	}

	stdout path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S3.launchd.seatbelt/startup.out
	stderr path = /private/var/folders/d8/hvxvltxn0fl4rmnd52sncbth0000gn/T/brokkr-seatbelt-probe-startup-48523.1/startup-48523-S3.launchd.seatbelt/startup.err
	inherited environment = {
		SSH_AUTH_SOCK => /var/run/com.apple.launchd.CFkKnLTePe/Listeners
	}

	default environment = {
		PATH => /usr/bin:/bin:/usr/sbin:/sbin
	}

	environment = {
		OSLogRateLimit => 64
		XPC_SERVICE_NAME => org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.2.S3.launchd.seatbelt
	}

	domain = gui/501 [100002]
	asid = 100002
	minimum runtime = 10
	exit timeout = 5
	runs = 1
	last exit code = 2

	resource coalition = {
		ID = 1243
		type = resource
		state = active
		active count = 1
		name = org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.2.S3.launchd.seatbelt
	}

	jetsam coalition = {
		ID = 1244
		type = jetsam
		state = active
		active count = 1
		name = org.brokkr.seatbelt.probe.startup-48523.1.48523.startup.2.S3.launchd.seatbelt
	}

	spawn type = daemon (3)
	jetsam priority = 40
	jetsam memory limit (active) = (unlimited)
	jetsam memory limit (inactive) = (unlimited)
	jetsamproperties category = daemon
	jetsam thread limit = 32
	cpumon = default

	properties = runatload | inferred program | system service | tle system
}"##;
