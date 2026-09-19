import assert from "node:assert/strict";
import { describe, test } from "node:test";
import {
	encodeBase64Bytes,
	encodeBase64Utf8,
	hasOnlyKeys,
	parseActivityLog,
	parseAdbDisablerState,
	parseCanonicalJson,
	parseKeyboxState,
	parseSoterSpoofState,
	parseSupportedAbi,
	shellQuote,
} from "../../src/cli_parse.ts";

describe("parseCanonicalJson", () => {
	test("accepts canonical JSON", () => {
		assert.deepEqual(parseCanonicalJson('{"a":1}', "x"), { a: 1 });
	});

	test("rejects non-JSON", () => {
		assert.throws(
			() => parseCanonicalJson("nope", "Keybox state"),
			/invalid Keybox state/,
		);
	});

	test("rejects any output that is not byte-identical to its round-trip", () => {
		// Key order survives JSON.stringify, so ordering is policed by
		// hasOnlyKeys instead; this guard catches formatting drift.
		assert.throws(() => parseCanonicalJson('{ "a":1}', "x"), /non-canonical/);
		assert.throws(() => parseCanonicalJson('{"a": 1}', "x"), /non-canonical/);
		assert.throws(() => parseCanonicalJson('{"a":1}\n', "x"), /non-canonical/);
		assert.deepEqual(parseCanonicalJson('{"b":1,"a":2}', "x"), { b: 1, a: 2 });
	});
});

describe("hasOnlyKeys", () => {
	test("requires the exact key set in order", () => {
		assert.equal(hasOnlyKeys({ a: 1, b: 2 }, ["a", "b"]), true);
		assert.equal(hasOnlyKeys({ a: 1, b: 2 }, ["b", "a"]), false);
		assert.equal(hasOnlyKeys({ a: 1 }, ["a", "b"]), false);
		assert.equal(hasOnlyKeys({ a: 1, b: 2, c: 3 }, ["a", "b"]), false);
	});
});

describe("parseSoterSpoofState", () => {
	test("accepts a canonical state", () => {
		assert.deepEqual(
			parseSoterSpoofState('{"enabled":true,"reboot_required":false}'),
			{ enabled: true, reboot_required: false },
		);
	});

	test("rejects a missing field", () => {
		assert.throws(
			() => parseSoterSpoofState('{"enabled":true}'),
			/invalid Soter spoof state/,
		);
	});

	test("rejects a non-boolean field", () => {
		assert.throws(
			() => parseSoterSpoofState('{"enabled":1,"reboot_required":false}'),
			/invalid Soter spoof state/,
		);
	});
});

describe("parseKeyboxState", () => {
	const valid = {
		valid: true,
		bundled: false,
		source: "google_hardware",
		level: "tee",
		play_integrity: "not_checked",
		revocation: "not_listed",
	};

	test("accepts a valid state", () => {
		assert.deepEqual(parseKeyboxState(JSON.stringify(valid)), valid);
	});

	test("accepts the bundled keybox only while valid", () => {
		const bundled = { ...valid, bundled: true };
		assert.deepEqual(parseKeyboxState(JSON.stringify(bundled)), bundled);
	});

	test("rejects bundled combined with invalid", () => {
		const bad = { ...valid, valid: false, bundled: true };
		assert.throws(
			() => parseKeyboxState(JSON.stringify(bad)),
			/invalid Keybox state/,
		);
	});

	test("rejects an unknown source", () => {
		const bad = { ...valid, source: "somewhere" };
		assert.throws(
			() => parseKeyboxState(JSON.stringify(bad)),
			/invalid Keybox state/,
		);
	});

	test("rejects an unknown revocation status", () => {
		const bad = { ...valid, revocation: "maybe" };
		assert.throws(
			() => parseKeyboxState(JSON.stringify(bad)),
			/invalid Keybox state/,
		);
	});
});

describe("parseActivityLog", () => {
	const entry = {
		action: "targets_saved",
		detail: "3",
		timestamp: 1_700_000_000,
	};

	test("accepts an entry", () => {
		assert.deepEqual(parseActivityLog(JSON.stringify([entry])), [entry]);
	});

	test("accepts an empty log", () => {
		assert.deepEqual(parseActivityLog("[]"), []);
	});

	test("rejects an unknown action", () => {
		const bad = { ...entry, action: "not_a_real_action" };
		assert.throws(
			() => parseActivityLog(JSON.stringify([bad])),
			/invalid WebUI activity entry/,
		);
	});

	test("rejects control characters in the detail", () => {
		const bad = { ...entry, detail: "a\u0001b" };
		assert.throws(
			() => parseActivityLog(JSON.stringify([bad])),
			/invalid WebUI activity entry/,
		);
	});

	test("rejects an out-of-range timestamp", () => {
		assert.throws(
			() => parseActivityLog(JSON.stringify([{ ...entry, timestamp: 0 }])),
			/invalid WebUI activity entry/,
		);
		assert.throws(
			() => parseActivityLog(JSON.stringify([{ ...entry, timestamp: 1e15 }])),
			/invalid WebUI activity entry/,
		);
	});

	test("rejects more entries than the log limit", () => {
		const many = Array.from({ length: 31 }, () => entry);
		assert.throws(
			() => parseActivityLog(JSON.stringify(many)),
			/invalid WebUI activity log/,
		);
	});
});

describe("parseAdbDisablerState", () => {
	test("accepts a canonical state", () => {
		const state = {
			enabled: true,
			dev_options: false,
			usb_debug: false,
			oem_unlock: false,
		};
		assert.deepEqual(parseAdbDisablerState(JSON.stringify(state)), state);
	});

	test("rejects a missing field", () => {
		assert.throws(
			() => parseAdbDisablerState('{"enabled":true}'),
			/invalid ADB Disabler state/,
		);
	});

	test("rejects malformed JSON", () => {
		assert.throws(
			() => parseAdbDisablerState("{"),
			/invalid ADB Disabler state/,
		);
	});
});

describe("parseSupportedAbi", () => {
	test("accepts the supported ABI list", () => {
		assert.equal(parseSupportedAbi("arm64-v8a"), "arm64-v8a");
		assert.equal(parseSupportedAbi("aarch64"), "arm64-v8a");
	});

	test("honours the order reported by the device", () => {
		assert.equal(parseSupportedAbi("arm64-v8a,armeabi-v7a"), "arm64-v8a");
	});

	test("rejects unsupported ABIs", () => {
		assert.equal(parseSupportedAbi("x86_64"), null);
		assert.equal(parseSupportedAbi(""), null);
	});
});

describe("base64 and shell quoting", () => {
	test("encodes UTF-8 payloads", () => {
		assert.equal(encodeBase64Utf8("abc"), "YWJj");
		assert.equal(
			encodeBase64Utf8("重构"),
			Buffer.from("重构", "utf8").toString("base64"),
		);
	});

	test("encodes raw bytes across the chunk boundary", () => {
		const bytes = new Uint8Array(0x8000 * 2 + 7).fill(65);
		assert.equal(
			encodeBase64Bytes(bytes),
			Buffer.from(bytes).toString("base64"),
		);
	});

	test("quotes single quotes for the device shell", () => {
		assert.equal(shellQuote("plain"), "'plain'");
		assert.equal(shellQuote("it's"), "'it'\\''s'");
	});
});
