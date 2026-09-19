import type { ExecResult } from "./bridge";
import { getBridge } from "./bridge";
import {
	type ActivityAction,
	type ActivityEntry,
	type AdbDisablerState,
	encodeBase64Bytes,
	encodeBase64Utf8,
	type KeyboxRevocationStatus,
	type KeyboxState,
	MAX_KEYBOX_XML_BYTES,
	parseActivityLog,
	parseAdbDisablerState,
	parseKeyboxState,
	parseSoterSpoofState,
	parseSupportedAbi,
	type SoterSpoofState,
	type SupportedAbi,
	shellQuote,
} from "./cli_parse";
import { normalizePackageNames } from "./package_name";
import {
	ANDROID_SECURITY_BULLETIN_MIRROR_URL,
	ANDROID_SECURITY_BULLETIN_URL,
	isSecurityPatchDate,
} from "./security_patch";

const MODULE_ROOT = "/data/adb/modules/oh_my_keymint";
const HOT_UPDATE_ROOT = "/data/adb/omk";
type HelperPaths = { abi: SupportedAbi; inject: string; keymint: string };
const KEYBOX_BASE64_CHUNK_BYTES = 48 * 1024;
const MAX_BULLETIN_BYTES = 2 * 1024 * 1024;
const MAX_SOTER_SPOOF_STATE_BYTES = 2 * 1024;

export type {
	ActivityAction,
	ActivityEntry,
	AdbDisablerState,
	KeyboxLevel,
	KeyboxRevocationStatus,
	KeyboxSource,
	KeyboxState,
	SoterSpoofState,
} from "./cli_parse";
export { MAX_KEYBOX_XML_BYTES };

export class Cli {
	#helperPaths: Promise<HelperPaths> | null = null;

	async getScoop(): Promise<string[]> {
		const output = await this.#runInject(["--webui-get-scoop"]);
		let parsed: unknown;
		try {
			parsed = JSON.parse(output);
		} catch {
			throw new Error("OMK returned an invalid package list");
		}
		return normalizePackageNames(parsed);
	}

	async setScoop(packages: string[]): Promise<void> {
		const normalized = normalizePackageNames(packages);
		const payload = encodeBase64Utf8(JSON.stringify(normalized));
		await this.#runInject(["--webui-set-scoop", payload]);
		await this.#recordActivity("targets_saved", String(normalized.length));
	}

	async installKeybox(contents: Uint8Array): Promise<void> {
		if (contents.byteLength > MAX_KEYBOX_XML_BYTES) {
			throw new Error(
				`keybox.xml exceeds the ${MAX_KEYBOX_XML_BYTES} byte limit`,
			);
		}

		const payload = encodeBase64Bytes(contents);
		const chunks: string[] = [];
		for (
			let offset = 0;
			offset < payload.length;
			offset += KEYBOX_BASE64_CHUNK_BYTES
		) {
			chunks.push(payload.slice(offset, offset + KEYBOX_BASE64_CHUNK_BYTES));
		}
		const { keymint } = await this.#getHelperPaths();
		await this.#run(keymint, ["--webui-install-keybox", ...chunks]);
		await this.#recordActivity("keybox_changed", "");
	}

	async getKeyboxState(): Promise<KeyboxState> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(keymint, ["--webui-get-keybox-state"], 256);
		return parseKeyboxState(output);
	}

	async checkKeyboxRevocation(): Promise<KeyboxRevocationStatus> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(
			keymint,
			["--webui-check-keybox-revocation"],
			256,
		);
		if (
			output !== "not_listed" &&
			output !== "suspended" &&
			output !== "revoked"
		) {
			throw new Error("OMK returned an invalid Keybox revocation status");
		}
		return output;
	}

	/** Apply ADB Disabler options and persist them for the next boot. */
	async setAdbDisabler(
		enabled: boolean,
		devOptions: boolean,
		usbDebug: boolean,
		oemUnlock: boolean,
	): Promise<void> {
		const { keymint } = await this.#getHelperPaths();
		const values = [enabled, devOptions, usbDebug, oemUnlock].map((value) =>
			value ? "1" : "0",
		);
		const output = await this.#run(
			keymint,
			["--webui-set-adb-disabler", ...values],
			256,
		);
		if (output !== "adb_disabler_applied") {
			throw new Error("OMK returned an unexpected ADB Disabler result");
		}
		await this.#recordActivity(
			"adb_disabler_changed",
			enabled ? "enabled" : "disabled",
		);
	}

	async getAdbDisabler(): Promise<AdbDisablerState> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(keymint, ["--webui-get-adb-disabler"], 256);
		return parseAdbDisablerState(output);
	}

	async syncSecurityPatch(date: string): Promise<string> {
		if (!isSecurityPatchDate(date)) {
			throw new Error("Invalid security-patch date");
		}

		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(keymint, [
			"--webui-sync-security-patch",
			date,
		]);
		const firstDayFallback = date.endsWith("-05")
			? `${date.slice(0, 8)}01`
			: null;
		if (output !== date && output !== firstDayFallback) {
			throw new Error("OMK returned an unexpected security-patch date");
		}
		await this.#recordActivity("security_patch_synced", output);
		return output;
	}

	async restoreDefaultSecurityPatch(): Promise<void> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(keymint, [
			"--webui-sync-security-patch",
			"auto",
		]);
		if (output !== "auto") {
			throw new Error("OMK returned an unexpected security-patch mode");
		}
		await this.#recordActivity("security_patch_restored", "");
	}

	async getSystemSecurityPatch(): Promise<string> {
		let probe: ExecResult;
		try {
			probe = await getBridge().exec(
				`/system/bin/sh -c ${shellQuote("/system/bin/getprop ro.build.version.security_patch")}`,
			);
		} catch (error) {
			throw new Error(
				`Unable to read the system security patch: ${error instanceof Error ? error.message : String(error)}`,
			);
		}
		if (probe.errno !== 0) {
			throw new Error(
				`Unable to read the system security patch: ${probe.stderr.trim() || `shell exited with code ${probe.errno}`}`,
			);
		}

		const patch = probe.stdout.trim();
		if (!isSecurityPatchDate(patch)) {
			throw new Error("Android returned an invalid system security-patch date");
		}
		return patch;
	}

	async getTeeStatus(): Promise<void> {
		const output = await this.#runInject(["--webui-get-tee-status"]);
		if (output !== "normal") {
			throw new Error("OMK returned an unexpected TEE status");
		}
	}

	async getActivityLog(): Promise<ActivityEntry[]> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(
			keymint,
			["--webui-get-activity-log"],
			16 * 1024,
		);
		return parseActivityLog(output);
	}

	async clearActivityLog(): Promise<void> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(
			keymint,
			["--webui-clear-activity-log"],
			256,
		);
		if (output !== "ok")
			throw new Error("OMK returned an unexpected activity-log result");
	}

	async fetchSecurityBulletin(): Promise<string> {
		let lastError: Error | null = null;
		for (const url of [
			ANDROID_SECURITY_BULLETIN_URL,
			ANDROID_SECURITY_BULLETIN_MIRROR_URL,
		]) {
			try {
				const { keymint } = await this.#getHelperPaths();
				return await this.#run(
					keymint,
					["--webui-fetch-security-bulletin", url],
					MAX_BULLETIN_BYTES + 1024,
				);
			} catch (error) {
				lastError = error instanceof Error ? error : new Error(String(error));
			}
		}
		throw new Error(
			`Unable to download the Android Security Bulletin: ${lastError?.message ?? "network request failed"}`,
		);
	}

	async getSoterSpoofState(): Promise<SoterSpoofState> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(
			keymint,
			["--webui-get-soter-spoof"],
			MAX_SOTER_SPOOF_STATE_BYTES,
		);
		return parseSoterSpoofState(output);
	}

	async setSoterSpoofEnabled(enabled: boolean): Promise<SoterSpoofState> {
		const { keymint } = await this.#getHelperPaths();
		const output = await this.#run(
			keymint,
			["--webui-set-soter-spoof", enabled ? "1" : "0"],
			MAX_SOTER_SPOOF_STATE_BYTES,
		);
		const state = parseSoterSpoofState(output);
		if (state.enabled !== enabled) {
			throw new Error("OMK did not apply the requested Soter spoof state");
		}
		await this.#recordActivity(
			enabled ? "soter_spoof_enabled" : "soter_spoof_disabled",
			"",
		);
		return state;
	}

	async #recordActivity(action: ActivityAction, detail: string): Promise<void> {
		try {
			const { keymint } = await this.#getHelperPaths();
			const encodedDetail = encodeBase64Utf8(detail);
			const output = await this.#run(
				keymint,
				["--webui-record-activity", action, encodedDetail],
				256,
			);
			if (output !== "ok")
				throw new Error("OMK returned an unexpected activity-log result");
		} catch (error) {
			// Activity history is supplementary and must not turn a completed operation into a failure.
			console.error("Unable to record WebUI activity:", error);
		}
	}

	async #runInject(args: string[]): Promise<string> {
		const { inject } = await this.#getHelperPaths();
		return this.#run(inject, args);
	}

	async #getHelperPaths(): Promise<HelperPaths> {
		if (this.#helperPaths !== null) return this.#helperPaths;

		const pending = this.#detectHelperPaths();
		this.#helperPaths = pending.catch((error) => {
			this.#helperPaths = null;
			throw error;
		});
		return this.#helperPaths;
	}

	async #detectHelperPaths(): Promise<HelperPaths> {
		let abiProbe: ExecResult;
		try {
			abiProbe = await getBridge().exec(
				`/system/bin/sh -c ${shellQuote("/system/bin/getprop ro.product.cpu.abilist; /system/bin/getprop ro.product.cpu.abi; /system/bin/uname -m 2>/dev/null || :")}`,
			);
		} catch (error) {
			throw new Error(
				`Unable to detect the Android ABI: ${error instanceof Error ? error.message : String(error)}`,
			);
		}
		if (abiProbe.errno !== 0) {
			throw new Error(
				`Unable to detect the Android ABI: ${abiProbe.stderr.trim() || `shell exited with code ${abiProbe.errno}`}`,
			);
		}

		const abi = parseSupportedAbi(abiProbe.stdout);
		if (abi === null) {
			throw new Error(
				"Unsupported Android ABI: OMK requires an arm64-v8a device",
			);
		}

		const roots = [HOT_UPDATE_ROOT, `${MODULE_ROOT}/libs/${abi}`];
		for (const root of roots) {
			const inject = `${root}/inject`;
			const keymint = `${root}/keymint`;
			const check = await getBridge().exec(
				`/system/bin/sh -c ${shellQuote(`[ -x ${shellQuote(inject)} ] && [ -x ${shellQuote(keymint)} ]`)}`,
			);
			if (check.errno === 0) return { abi, inject, keymint };
		}

		throw new Error(`OMK ${abi} helper binaries are not installed`);
	}

	#run(
		binary: string,
		args: string[],
		maxOutputBytes = Number.POSITIVE_INFINITY,
	): Promise<string> {
		return new Promise((resolve, reject) => {
			let stdout = "";
			let stderr = "";
			let stdoutBytes = 0;
			let stderrBytes = 0;
			let outputTooLarge = false;
			let settled = false;
			const process = getBridge().spawn(binary, args);

			process.stdout.on("data", (chunk: string) => {
				if (outputTooLarge) return;
				stdoutBytes += new TextEncoder().encode(chunk).byteLength;
				if (stdoutBytes > maxOutputBytes) {
					outputTooLarge = true;
					return;
				}
				stdout += chunk;
			});
			process.stderr.on("data", (chunk: string) => {
				if (stderrBytes >= 8192) return;
				const remaining = 8192 - stderrBytes;
				const encoded = new TextEncoder().encode(chunk);
				stderrBytes += encoded.byteLength;
				stderr += new TextDecoder().decode(encoded.subarray(0, remaining));
			});
			process.on("exit", (code: number | null) => {
				if (settled) return;
				settled = true;
				if (outputTooLarge) {
					reject(new Error("command output exceeds the configured limit"));
				} else if (code === 0) {
					resolve(stdout.trim());
				} else {
					reject(
						new Error(
							stderr.trim() ||
								`OMK helper exited with code ${code ?? "unknown"}`,
						),
					);
				}
			});
			process.on("error", (error: Error) => {
				if (settled) return;
				settled = true;
				reject(new Error(`Unable to run the OMK helper: ${error.message}`));
			});
		});
	}
}
