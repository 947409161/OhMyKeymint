export const SUPPORTED_ABIS = ["arm64-v8a"] as const;
export type SupportedAbi = (typeof SUPPORTED_ABIS)[number];

const MAX_ACTIVITY_ENTRIES = 30;
const MAX_ACTIVITY_DETAIL_BYTES = 256;
const MAX_ACTIVITY_TIMESTAMP = 253_402_300_799;

const ACTIVITY_ACTIONS = [
	"targets_saved",
	"keybox_changed",
	"widevine_installed",
	"security_patch_synced",
	"security_patch_restored",
	"soter_spoof_enabled",
	"soter_spoof_disabled",
	"adb_disabler_changed",
] as const;
export type ActivityAction = (typeof ACTIVITY_ACTIONS)[number];

export interface ActivityEntry {
	action: ActivityAction;
	detail: string;
	timestamp: number;
}

export const MAX_KEYBOX_XML_BYTES = 64 * 1024;

export interface SoterSpoofState {
	enabled: boolean;
	/**
	 * True while the packaged payload still disagrees with `enabled`, i.e. until
	 * the next boot lets the module's boot script act on the setting. The dialog
	 * must not present the switch as taking effect before then.
	 */
	reboot_required: boolean;
}

export type KeyboxSource = "google_hardware" | "google_remote" | "unknown";
export type KeyboxLevel = "tee" | "strongbox" | "unknown";
export interface AdbDisablerState {
	enabled: boolean;
	dev_options: boolean;
	usb_debug: boolean;
	oem_unlock: boolean;
}
export type PlayIntegrityStatus = "not_checked";
export type KeyboxRevocationStatus =
	| "not_checked"
	| "checking"
	| "not_listed"
	| "suspended"
	| "revoked"
	| "unknown";

export interface KeyboxState {
	valid: boolean;
	bundled: boolean;
	source: KeyboxSource;
	level: KeyboxLevel;
	play_integrity: PlayIntegrityStatus;
	revocation: KeyboxRevocationStatus;
}

export function parseCanonicalJson(
	output: string,
	description: string,
): unknown {
	let parsed: unknown;
	try {
		parsed = JSON.parse(output);
	} catch {
		throw new Error(`OMK returned invalid ${description}`);
	}
	if (JSON.stringify(parsed) !== output) {
		throw new Error(`OMK returned non-canonical ${description}`);
	}
	return parsed;
}

export function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function hasOnlyKeys(
	value: Record<string, unknown>,
	allowed: readonly string[],
): boolean {
	const keys = Object.keys(value);
	return (
		keys.length === allowed.length &&
		keys.every((key, index) => key === allowed[index])
	);
}

export function parseSoterSpoofState(output: string): SoterSpoofState {
	const parsed = parseCanonicalJson(output, "Soter spoof state");
	if (
		!isRecord(parsed) ||
		!hasOnlyKeys(parsed, ["enabled", "reboot_required"]) ||
		typeof parsed.enabled !== "boolean" ||
		typeof parsed.reboot_required !== "boolean"
	) {
		throw new Error("OMK returned an invalid Soter spoof state");
	}
	return { enabled: parsed.enabled, reboot_required: parsed.reboot_required };
}

export function parseKeyboxState(output: string): KeyboxState {
	const parsed = parseCanonicalJson(output, "Keybox state");
	if (
		!isRecord(parsed) ||
		!hasOnlyKeys(parsed, [
			"valid",
			"bundled",
			"source",
			"level",
			"play_integrity",
			"revocation",
		]) ||
		typeof parsed.valid !== "boolean" ||
		typeof parsed.bundled !== "boolean" ||
		(parsed.source !== "google_hardware" &&
			parsed.source !== "google_remote" &&
			parsed.source !== "unknown") ||
		(parsed.level !== "tee" &&
			parsed.level !== "strongbox" &&
			parsed.level !== "unknown") ||
		parsed.play_integrity !== "not_checked" ||
		(parsed.revocation !== "not_checked" &&
			parsed.revocation !== "not_listed" &&
			parsed.revocation !== "suspended" &&
			parsed.revocation !== "revoked" &&
			parsed.revocation !== "unknown") ||
		(!parsed.valid && parsed.bundled)
	) {
		throw new Error("OMK returned an invalid Keybox state");
	}
	return {
		valid: parsed.valid,
		bundled: parsed.bundled,
		source: parsed.source,
		level: parsed.level,
		play_integrity: parsed.play_integrity,
		revocation: parsed.revocation,
	};
}

export function parseActivityLog(output: string): ActivityEntry[] {
	const parsed = parseCanonicalJson(output, "WebUI activity log");
	if (!Array.isArray(parsed) || parsed.length > MAX_ACTIVITY_ENTRIES) {
		throw new Error("OMK returned an invalid WebUI activity log");
	}

	const actions = new Set<string>(ACTIVITY_ACTIONS);
	return parsed.map((value) => {
		if (
			!isRecord(value) ||
			!hasOnlyKeys(value, ["action", "detail", "timestamp"]) ||
			typeof value.action !== "string" ||
			!actions.has(value.action) ||
			typeof value.detail !== "string" ||
			new TextEncoder().encode(value.detail).byteLength >
				MAX_ACTIVITY_DETAIL_BYTES ||
			/\p{Cc}/u.test(value.detail) ||
			typeof value.timestamp !== "number" ||
			!Number.isSafeInteger(value.timestamp) ||
			value.timestamp <= 0 ||
			value.timestamp > MAX_ACTIVITY_TIMESTAMP
		) {
			throw new Error("OMK returned an invalid WebUI activity entry");
		}
		return {
			action: value.action as ActivityAction,
			detail: value.detail,
			timestamp: value.timestamp,
		};
	});
}

export function encodeBase64Bytes(bytes: Uint8Array): string {
	let binary = "";
	const chunkSize = 0x8000;
	for (let offset = 0; offset < bytes.length; offset += chunkSize) {
		binary += String.fromCharCode(
			...bytes.subarray(offset, offset + chunkSize),
		);
	}
	return btoa(binary);
}

export function encodeBase64Utf8(value: string): string {
	return encodeBase64Bytes(new TextEncoder().encode(value));
}

export function shellQuote(value: string): string {
	return `'${value.replace(/'/g, `'\\''`)}'`;
}

export function normalizeAbiToken(value: string): SupportedAbi | null {
	switch (value.trim()) {
		case "arm64-v8a":
		case "aarch64":
			return "arm64-v8a";
		default:
			return null;
	}
}

export function parseSupportedAbi(output: string): SupportedAbi | null {
	const tokens = output.split(/[\s,]+/).filter(Boolean);
	for (const token of tokens) {
		const abi = normalizeAbiToken(token);
		if (abi !== null) return abi;
	}
	return null;
}

export function parseAdbDisablerState(output: string): AdbDisablerState {
	let parsed: unknown;
	try {
		parsed = JSON.parse(output);
	} catch {
		throw new Error("OMK returned invalid ADB Disabler state");
	}
	if (
		!isRecord(parsed) ||
		!hasOnlyKeys(parsed, [
			"enabled",
			"dev_options",
			"usb_debug",
			"oem_unlock",
		]) ||
		typeof parsed.enabled !== "boolean" ||
		typeof parsed.dev_options !== "boolean" ||
		typeof parsed.usb_debug !== "boolean" ||
		typeof parsed.oem_unlock !== "boolean"
	) {
		throw new Error("OMK returned invalid ADB Disabler state");
	}
	return {
		enabled: parsed.enabled,
		dev_options: parsed.dev_options,
		usb_debug: parsed.usb_debug,
		oem_unlock: parsed.oem_unlock,
	};
}
