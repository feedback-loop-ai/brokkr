import { createRequire } from "node:module";
import { randomUUID } from "node:crypto";
import z from "@deepseek-ai/schemastery";
import { pathToFileURL } from "node:url";
//#region src/host-modules.ts
/**
* Late-bound access to the harness packages this runner drives.
*
* These live in the profile's own tree, not in this plugin's. A plugin
* installed normally finds them by Node's parent walk, but one installed with
* `link:` (the usual dev loop) sits outside the profile directory and never
* reaches it. Resolving through `ctx.baseUrl` — the profile directory the
* loader booted from — covers both, and importing the resolved path keeps a
* single module instance shared with the host rather than a second copy.
*
* @module dsh-plugin-cli-session/host-modules
*/
/**
* Resolve one specifier against this module, then against the profile.
* @param specifier - bare package specifier.
* @param baseUrl - the loader's base URL, when the entry has one.
* @returns an absolute file URL for the module.
* @throws when no anchor resolves it, naming every path tried.
*/
function resolveHostModule(specifier, baseUrl) {
	const failures = [];
	for (const anchor of [import.meta.url, ...baseUrl === void 0 ? [] : [baseUrl]]) try {
		return pathToFileURL(createRequire(anchor).resolve(specifier)).href;
	} catch (error) {
		failures.push(`${anchor}: ${error instanceof Error ? error.message.split("\n")[0] ?? "" : String(error)}`);
	}
	throw new Error(`cli-session: cannot resolve "${specifier}" from the profile. Tried:\n` + failures.map((line) => `  - ${line}`).join("\n"));
}
/**
* Load the harness modules the runner needs.
* @param baseUrl - `ctx.baseUrl` of the runner entry.
* @returns the resolved harness entry points.
*/
async function loadHostModules(baseUrl) {
	const [agent, llm, session] = await Promise.all([
		import(resolveHostModule("@deepseek-ai/dsh-agent", baseUrl)),
		import(resolveHostModule("@deepseek-ai/dsh-llm", baseUrl)),
		import(resolveHostModule("@deepseek-ai/dsh-session", baseUrl))
	]);
	return {
		installModelSelection: agent.installModelSelection,
		createUserMessage: llm.createUserMessage,
		SessionId: session.SessionId
	};
}
//#endregion
//#region src/index.ts
/**
* CLI runner: one resume-capable agent turn, printed either as conversational
* text or as a machine-readable envelope.
*
* This is the merge of two hand-copied profile runners that had drifted into
* near-duplicates — `summarize`, session resolution, exit handling and the
* whole drive loop were identical. What actually differed is now configuration:
* the session tag, whether the session id is announced, and the output shape.
*
* @module dsh-plugin-cli-session
*/
/** Cordis plugin name. */
const name = "cli-runner";
/** Core services required before the turn can start. */
const inject = [
	"agentDefaultModel",
	"agents",
	"sessions",
	"sessionPersistence"
];
/** Runtime schema for {@link Config}. */
const Config = z.object({
	request: z.object({
		action: z.string().default("new"),
		task: z.string().default(""),
		sessionId: z.string().default(""),
		outputFormat: z.string().default("text"),
		jsonSchema: z.string().default("")
	}),
	sessionTag: z.string().default("cli"),
	announceSessionId: z.boolean().default(false),
	exitGraceMs: z.number().default(1500)
});
/** Last assistant text plus the turn outcome, over one owned interval. */
function summarize(events, firstSeq) {
	let started = false;
	let text = "";
	let reason;
	for (const event of events) {
		if (event.seq < firstSeq) continue;
		if (event.type === "turn/start") {
			started = true;
			continue;
		}
		if (!started) continue;
		if (event.type === "assistant/message") {
			const joined = (event.data.message?.content ?? []).filter((block) => block.type === "text").map((block) => block.text ?? "").join("");
			if (joined !== "") text = joined;
		}
		if (event.type === "turn/end") reason = event.data.reason;
	}
	return {
		text,
		reason
	};
}
/** The latest provider-reported usage in this run, when present. */
function collectUsage(events, firstSeq) {
	let usage;
	for (const event of events) {
		if (event.seq < firstSeq || event.type !== "assistant/message") continue;
		const candidate = event.data.usage;
		if (candidate !== void 0) usage = candidate;
	}
	return usage;
}
/** Turns started in this run. */
function countTurns(events, firstSeq) {
	let count = 0;
	for (const event of events) if (event.seq >= firstSeq && event.type === "turn/start") count += 1;
	return count;
}
function sessionsInCwd(headers, cwd, tag) {
	return headers.filter((header) => header.cwd === cwd && header.agentPreset === tag).sort((a, b) => b.createdAt - a.createdAt);
}
function normalizeSessionId(id) {
	return id.startsWith("session-") ? id : `session-${id}`;
}
/** Ask for a graceful shutdown, with a hard fallback so a one-shot CLI always exits. */
function exitNow(io, code) {
	io.exit(code);
	setTimeout(() => {
		process.exit(code);
	}, io.graceMs);
}
function fail(io, error) {
	io.stderr.write(`dsh: ${error instanceof Error ? error.message : String(error)}\n`);
	exitNow(io, 1);
}
/**
* Reject a configuration that cannot do anything sensible.
*
* The row's `request` comes from `!!js ctx.cliStartup`; wire it wrong and the
* schema's defaults quietly produce `action: 'new'` with an empty task, which
* would create a session and spend a model call on nothing. A blank session tag
* is just as bad: it is the label `--list` and `--resume` scope by.
* @param config - the row's resolved configuration.
* @throws when the configuration cannot be run.
*/
function assertRunnable(config) {
	if (config.sessionTag.trim() === "") throw new Error("cli-runner: sessionTag must not be empty; it is the label --list and --resume scope by");
	if (!Number.isFinite(config.exitGraceMs) || config.exitGraceMs < 0) throw new Error(`cli-runner: exitGraceMs must be a non-negative number, got ${String(config.exitGraceMs)}`);
	if (config.request.action !== "list" && config.request.task.trim() === "") throw new Error("cli-runner: no task to run. The row's config.request should be wired to the startup service, as in `request: !!js ctx.cliStartup`.");
}
/**
* Compose the task text.
*
* `--json-schema` is a prompt-level request, not a provider structured-output
* constraint: the schema is appended as an instruction, so a model can still
* answer with fenced or prose-wrapped JSON. Callers must parse defensively.
* @param request - the parsed invocation.
* @returns the text to send as the user turn.
*/
function buildTask(request) {
	if (request.jsonSchema === "") return request.task;
	return `${request.task}\n\nRespond with a single valid JSON object that conforms EXACTLY to this JSON Schema. Output ONLY the JSON itself — no markdown code fences, no commentary, no surrounding text:\n${request.jsonSchema}`;
}
async function run(ctx, config, io) {
	const request = config.request;
	await ctx.get("loader")?.await();
	const agents = ctx.get("agents");
	const sessions = ctx.get("sessions");
	const persistence = ctx.get("sessionPersistence");
	const defaultModel = ctx.get("agentDefaultModel");
	if (agents === void 0 || sessions === void 0 || persistence === void 0 || defaultModel === void 0) return;
	const cwd = process.cwd();
	if (request.action === "list") {
		const mine = sessionsInCwd(await persistence.list(), cwd, config.sessionTag);
		if (request.outputFormat === "text") {
			if (mine.length === 0) io.stdout.write(`(no ${config.sessionTag} sessions for ${cwd})\n`);
			else for (const header of mine) io.stdout.write(`${header.id}\t${new Date(header.createdAt).toISOString()}\t${header.cwd ?? ""}\n`);
		} else io.stdout.write(`${JSON.stringify({ sessions: mine.map((header) => ({
			id: header.id,
			created_at: new Date(header.createdAt).toISOString(),
			cwd: header.cwd ?? ""
		})) })}\n`);
		exitNow(io, 0);
		return;
	}
	const { installModelSelection, createUserMessage, SessionId } = await loadHostModules(ctx.baseUrl);
	const startTime = Date.now();
	const selection = defaultModel.currentSelection();
	const setup = (agentCtx) => {
		installModelSelection(agentCtx, {
			current: selection,
			assembled: void 0
		});
	};
	const agentOptions = {
		provider: selection.provider,
		model: selection.model
	};
	let handle;
	let sessionId;
	if (request.action === "new") {
		sessionId = `session-${randomUUID()}`;
		handle = await agents.create({
			sessionId: SessionId(sessionId),
			meta: {
				cwd,
				agentPreset: config.sessionTag
			},
			agentOptions,
			setup
		});
	} else {
		const resolved = request.action === "resume-session" ? normalizeSessionId(request.sessionId) : sessionsInCwd(await persistence.list(), cwd, config.sessionTag)[0]?.id;
		if (resolved === void 0) {
			io.stderr.write(`dsh: no ${config.sessionTag} session to resume in ${cwd}\n`);
			exitNow(io, 1);
			return;
		}
		sessionId = resolved;
		handle = await agents.resume({
			resumeSessionId: SessionId(sessionId),
			agentOptions,
			setup
		});
	}
	const { agent } = handle;
	if (config.announceSessionId) io.stderr.write(`session: ${sessionId}\n`);
	await agent.whenIdle();
	const firstSeq = agent.session.seq;
	if (request.outputFormat === "stream-json") io.stdout.write(`${JSON.stringify({
		type: "system",
		subtype: "init",
		session_id: sessionId,
		cwd,
		provider: selection.provider,
		model: selection.model
	})}\n`);
	agent.followup(createUserMessage({
		content: [{
			type: "text",
			text: buildTask(request)
		}],
		source: { kind: "user" }
	}));
	await agent.whenIdle();
	await sessions.flush(agent.session);
	const events = agent.session.snapshotEvents(firstSeq);
	const outcome = summarize(events, firstSeq);
	const usage = collectUsage(events, firstSeq);
	const isError = outcome.reason?.kind !== "completed";
	if (request.outputFormat === "text") io.stdout.write(`${outcome.text}\n`);
	else {
		const result = {
			type: "result",
			subtype: isError ? "error" : "success",
			is_error: isError,
			duration_ms: Date.now() - startTime,
			num_turns: countTurns(events, firstSeq),
			result: isError ? outcome.reason?.error?.message ?? "" : outcome.text,
			session_id: sessionId
		};
		if (usage !== void 0) result.usage = usage;
		io.stdout.write(`${JSON.stringify(result)}\n`);
	}
	if (isError) {
		const error = outcome.reason?.error;
		io.stderr.write(`dsh: ${error?.code ?? "error"}: ${error?.message ?? ""}\n`);
	}
	exitNow(io, isError ? 1 : 0);
}
/** Drive one turn and exit. */
function apply(ctx, config) {
	assertRunnable(config);
	const exit = ctx.get("appExit");
	if (exit === void 0) throw new Error("cli-runner: the launcher must provide ctx.appExit before the tree mounts");
	const io = {
		stdout: process.stdout,
		stderr: process.stderr,
		exit,
		graceMs: config.exitGraceMs
	};
	run(ctx, config, io).catch((error) => {
		fail(io, error);
	});
}
//#endregion
export { Config, apply, assertRunnable, buildTask, collectUsage, countTurns, inject, name, normalizeSessionId, sessionsInCwd, summarize };

//# sourceMappingURL=index.js.map