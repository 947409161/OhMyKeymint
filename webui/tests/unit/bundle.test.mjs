import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { describe, test } from "node:test";
import { fileURLToPath } from "node:url";

/*
 * Release-build guard.
 *
 * src/bridge/dev.ts fabricates device state so development and Playwright can
 * run the production code path. If it ever reached a release build, a device
 * without a working KernelSU bridge would be shown a healthy-looking keybox
 * and package list that no native call produced — the worst failure mode this
 * product has, because the user would believe an operation succeeded.
 *
 * The markers below are string literals that survive minification and appear
 * nowhere else in src/, so their absence proves the module was dropped.
 *
 * Run `pnpm build` first; this suite reads template/webroot/assets/*.js.
 */
const assetsDir = fileURLToPath(
	new URL("../../../template/webroot/assets/", import.meta.url),
);

const DEV_BRIDGE_MARKERS = [
	"unknown keymint command",
	"unknown inject command",
	"org.telegram.messenger",
];

function readBuiltScripts() {
	let files;
	try {
		files = readdirSync(assetsDir).filter((name) => name.endsWith(".js"));
	} catch {
		files = [];
	}
	assert.ok(
		files.length > 0,
		"Built scripts not found — run `pnpm build` before `pnpm test`.",
	);
	return files.map((name) => ({
		name,
		source: readFileSync(assetsDir + name, "utf8"),
	}));
}

describe("release build", () => {
	const scripts = readBuiltScripts();

	test("does not ship the development bridge", () => {
		const leaks = [];
		for (const { name, source } of scripts) {
			for (const marker of DEV_BRIDGE_MARKERS) {
				if (source.includes(marker)) leaks.push(`${name}: ${marker}`);
			}
		}
		assert.deepEqual(
			leaks,
			[],
			`The development bridge reached a release build: ${leaks.join(", ")}. Guard its install with import.meta.env.DEV.`,
		);
	});

	test("has no unresolved build-time environment references", () => {
		for (const { name, source } of scripts) {
			assert.ok(
				!source.includes("import.meta.env"),
				`${name} still contains import.meta.env, which Vite should have replaced at build time`,
			);
		}
	});

	test("keeps the real bridge and its protocol in the bundle", () => {
		// The counterpart to the check above: dropping the dev bridge must not
		// have dropped the strings the native helper protocol depends on.
		const combined = scripts.map((script) => script.source).join("\n");
		for (const marker of ["--webui-get-keybox-state", "adb_disabler_applied"]) {
			assert.ok(
				combined.includes(marker),
				`${marker} is missing from the release build`,
			);
		}
	});
});
