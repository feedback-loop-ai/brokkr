// brokkr-console-probe, browser half. Hand-written in the shape dsh's tsdown
// preset emits for every first-party client bundle (see
// node_modules/@deepseek-ai/dsh-client-ui-schedule/lib/client.js): executing
// the script only registers a lazy CJS factory with window.__ModuleLoader__;
// `require` resolves against the shell's frozen platform table (react, ...).
window.__ModuleLoader__.load({
	id: "brokkr-console-probe",
	factory: (require) => {
		var module = { exports: {} };
		var exports = module.exports;
		Object.defineProperty(exports, Symbol.toStringTag, { value: "Module" });
		const React = require("react");
		const { jsx, jsxs } = require("react/jsx-runtime");

		const PANEL_ID = "brokkr-probe";
		const report = (event, detail) => {
			const q = new URLSearchParams({ event });
			if (detail !== undefined) q.set("detail", String(detail));
			fetch("/brokkr-probe/report?" + q.toString()).catch(() => {});
		};

		function ProbePage() {
			const [ping, setPing] = React.useState("pending");
			const [ws, setWs] = React.useState("pending");
			React.useEffect(() => {
				report("main-mounted", PANEL_ID);
				fetch("/brokkr-probe/ping").then((r) => r.json()).then((j) => {
					setPing(JSON.stringify(j));
					report("main-ping-ok", j.route);
				}, (e) => setPing("error " + e));
				const sock = new WebSocket(location.origin.replace(/^http/, "ws") + "/brokkr-probe/ws");
				const seen = [];
				sock.onmessage = (m) => {
					seen.push(m.data);
					setWs(seen.join(" | "));
					if (seen.length === 1) sock.send("from-main-panel");
					else { report("main-ws-echo", m.data); sock.close(); }
				};
				sock.onerror = () => setWs("error");
				return () => sock.close();
			}, []);
			return jsxs("div", {
				"data-testid": "brokkr-probe-page",
				style: { padding: 24, fontFamily: "monospace" },
				children: [
					jsx("h1", { children: "Brokkr console probe" }),
					jsx("p", { "data-testid": "brokkr-probe-ping", children: "ping: " + ping }),
					jsx("p", { "data-testid": "brokkr-probe-ws", children: "ws: " + ws })
				]
			});
		}

		function ProbeIcon({ size, active }) {
			React.useEffect(() => { report("sidebar-icon-mounted", "size=" + size); }, []);
			return jsx("span", {
				"data-testid": "brokkr-probe-icon",
				"data-active": String(active),
				style: { display: "inline-block", width: size, height: size, lineHeight: size + "px", textAlign: "center", fontWeight: 700 },
				children: "B"
			});
		}

		const name = "brokkr-console-probe";
		const inject = ["slots"];
		function apply(ctx) {
			report("client-apply");
			ctx.slots.inject("main", () => ctx.slots.register({
				name: "main",
				key: PANEL_ID
			}, ProbePage));
			ctx.slots.inject("sidebar.panellist", () => ctx.slots.register({
				name: "sidebar.panellist",
				id: PANEL_ID,
				order: 50,
				label: () => "Brokkr probe"
			}, ProbeIcon));
			report("client-registered", "main+sidebar.panellist");
		}
		exports.apply = apply;
		exports.inject = inject;
		exports.name = name;
		return module.exports;
	}
});
