import type {
	ActivityEntry,
	KeyboxLevel,
	KeyboxRevocationStatus,
	KeyboxSource,
} from "../cli";
import { cli } from "./app";

export type KeyboxStatus =
	| "loading"
	| "custom"
	| "bundled"
	| "invalid"
	| "error";
export type TeeStatus = "loading" | "normal" | "error";
export type ModuleStatus = "loading" | "ready" | "error";

export interface Identity {
	moduleStatus: ModuleStatus;
	keyboxStatus: KeyboxStatus;
	keyboxSource: KeyboxSource;
	keyboxLevel: KeyboxLevel;
	keyboxRevocation: KeyboxRevocationStatus;
	teeStatus: TeeStatus;
	securityPatch: string | null;
	soterSpoofEnabled: boolean | undefined;
	selectedCount: number;
}

export interface ActivityResult {
	status: "loading" | "ready" | "error";
	entries: ActivityEntry[];
}

export interface LoadedIdentity {
	identity: Identity;
	activity: ActivityResult;
}

async function loadActivity(): Promise<ActivityResult> {
	try {
		return { status: "ready", entries: await cli.getActivityLog() };
	} catch (error) {
		console.error("Unable to load the WebUI activity log:", error);
		return { status: "error", entries: [] };
	}
}

/**
 * Reads the whole identity surface. Each probe is settled independently: one
 * unavailable backend must not blank the other fields, because a device can
 * legitimately report no keybox while still answering for TEE and patch level.
 */
export async function loadIdentity(
	selectedCount: number,
): Promise<LoadedIdentity> {
	const [keybox, patch, tee, soter, activity] = await Promise.all([
		cli.getKeyboxState().then(
			(value) => ({ ok: true as const, value }),
			() => ({ ok: false as const }),
		),
		cli.getSystemSecurityPatch().then(
			(value) => ({ ok: true as const, value }),
			() => ({ ok: false as const }),
		),
		cli.getTeeStatus().then(
			() => ({ ok: true as const }),
			() => ({ ok: false as const }),
		),
		cli.getSoterSpoofState().then(
			(value) => ({ ok: true as const, value }),
			() => ({ ok: false as const }),
		),
		loadActivity(),
	]);

	let keyboxStatus: KeyboxStatus = "error";
	let keyboxSource: KeyboxSource = "unknown";
	let keyboxLevel: KeyboxLevel = "unknown";
	let keyboxRevocation: KeyboxRevocationStatus = "not_checked";

	if (keybox.ok) {
		const value = keybox.value;
		keyboxStatus = value.valid
			? value.bundled
				? "bundled"
				: "custom"
			: "invalid";
		keyboxSource = value.source;
		keyboxLevel = value.level;
		keyboxRevocation = value.valid ? "checking" : value.revocation;
		if (value.valid) {
			try {
				keyboxRevocation = await cli.checkKeyboxRevocation();
			} catch {
				keyboxRevocation = "unknown";
			}
		}
	}

	return {
		identity: {
			moduleStatus: "ready",
			keyboxStatus,
			keyboxSource,
			keyboxLevel,
			keyboxRevocation,
			teeStatus: tee.ok ? "normal" : "error",
			securityPatch: patch.ok ? patch.value : null,
			soterSpoofEnabled: soter.ok ? soter.value.enabled : undefined,
			selectedCount,
		},
		activity,
	};
}
