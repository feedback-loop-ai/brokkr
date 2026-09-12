import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { Command } from "commander";
import { parseCmdline } from "@deepseek-ai/dsh-cmdline";
//#region src/startup.ts
/**
* CLI startup: parse the flags the launcher forwards, then publish them as the
* `cliStartup` service the runner injects.
*
* @module dsh-plugin-cli-session/startup
*/
/** Cordis plugin name. */
const name = "cli-startup";
/** Services required before this plugin can parse the command line. */
const inject = ["cmdlineArgs"];
/** Service key this plugin provides. */
const CLI_STARTUP_SERVICE = "cliStartup";
/** Output shapes the runner can emit. */
const OUTPUT_FORMATS = [
	"text",
	"json",
	"stream-json"
];
function cliCommand() {
	return new Command().name("dsh --profile <name>").description("Run a task in a resumable session; print conversational text or machine-readable JSON.").helpOption("-h, --help", "show this help").argument("[task...]", "the task text; multiple words are joined by spaces").option("-o, --output-format <format>", `output format: ${OUTPUT_FORMATS.join(", ")}`, "text").option("--json-schema <schema>", "JSON Schema the response should conform to (asked for in the prompt)").option("-n, --new", "start a new session (this is the default)").option("-r, --resume", "resume the most recent session of this profile in the current working directory").option("-s, --session <id>", "resume a specific session id").option("-w, --workdir <dir>", "working directory for a NEW session (chdir before creating)").option("-l, --list", "list this profile's sessions in the current working directory and exit").showHelpAfterError("(dsh-plugin-cli-session owns this command line; install it in a headless-style profile)").addHelpText("after", [
		"",
		"Examples:",
		"  dsh --profile chat \"explain this file\"",
		"  dsh --profile chat --resume \"follow-up\"",
		"  dsh --profile api -o json \"explain this file\"",
		"  dsh --profile api -o json --json-schema '{\"type\":\"object\"}' \"summarize this file\"",
		""
	].join("\n"));
}
/** Parse argv and publish the request. */
function apply(ctx) {
	const program = cliCommand();
	program.action(() => {
		const opts = program.opts();
		const outputFormat = opts.outputFormat ?? "text";
		if (!OUTPUT_FORMATS.includes(outputFormat)) program.error(`error: --output-format must be one of: ${OUTPUT_FORMATS.join(", ")}`);
		if (opts.list === true) {
			ctx.provide(CLI_STARTUP_SERVICE, {
				action: "list",
				task: "",
				sessionId: "",
				outputFormat,
				jsonSchema: ""
			});
			return;
		}
		const task = program.args.join(" ").trim();
		if (task === "") program.error("error: a task is required, e.g. dsh --profile chat \"your task\"");
		if (opts.session !== void 0 && opts.resume === true) program.error("error: --session and --resume are mutually exclusive");
		if (opts.new === true && (opts.session !== void 0 || opts.resume === true)) program.error("error: --new cannot be combined with --session/--resume");
		if (opts.workdir !== void 0 && (opts.session !== void 0 || opts.resume === true)) program.error("error: --workdir applies to a new session only; a resumed session keeps its own cwd");
		if (opts.workdir !== void 0) {
			const dir = resolve(opts.workdir);
			mkdirSync(dir, { recursive: true });
			process.chdir(dir);
		}
		ctx.provide(CLI_STARTUP_SERVICE, {
			action: opts.session !== void 0 ? "resume-session" : opts.resume === true ? "resume-last" : "new",
			task,
			sessionId: opts.session ?? "",
			outputFormat,
			jsonSchema: opts.jsonSchema ?? ""
		});
	});
	parseCmdline(ctx, program);
}
//#endregion
export { CLI_STARTUP_SERVICE, OUTPUT_FORMATS, apply, inject, name };

//# sourceMappingURL=startup.js.map