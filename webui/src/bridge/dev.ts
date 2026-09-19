import type {
	Bridge,
	ChildProcess,
	ExecOptions,
	ExecResult,
	PackagesInfo,
	Stdio,
} from "../bridge";

/*
 * A deterministic stand-in for the KernelSU bridge, installed whenever the
 * WebUI runs outside a WebView (vite dev, Playwright, a desktop browser).
 *
 * It answers on the `--webui-*` protocol with canonical JSON, exactly as the
 * real helpers do, so the application runs its production code path in
 * development instead of through scattered `isDev()` branches. Errors are
 * modelled too, so the UI's failure states are reachable without a device.
 */

const ABI = "arm64-v8a";
const SECURITY_PATCH = "2026-09-05";

const SAMPLE_PACKAGES = [
	"com.android.vending",
	"com.google.android.gms",
	"com.google.android.gsf",
	"io.github.vvb2060.keyattestation",
	"org.telegram.messenger",
];

const SAMPLE_SYSTEM_PACKAGES = [
	"com.android.settings",
	"com.android.systemui",
	"com.google.android.gms",
	"com.google.android.gsf",
	"com.android.vending",
];

const SAMPLE_LABELS: Record<string, string> = {
	"com.android.vending": "Google Play Store",
	"com.google.android.gms": "Google Play services",
	"com.google.android.gsf": "Google Services Framework",
	"com.android.settings": "Settings",
	"com.android.systemui": "System UI",
	"io.github.vvb2060.keyattestation": "Key Attestation",
	"org.telegram.messenger": "Telegram",
};

interface MutableState {
	scoop: string[];
	keyboxInstalled: boolean;
	soterSpoof: boolean;
	adb: {
		enabled: boolean;
		dev_options: boolean;
		usb_debug: boolean;
		oem_unlock: boolean;
	};
	activity: Array<{ action: string; detail: string; timestamp: number }>;
}

const state: MutableState = {
	scoop: ["io.github.vvb2060.keyattestation", "com.google.android.gms"],
	keyboxInstalled: true,
	soterSpoof: false,
	adb: {
		enabled: false,
		dev_options: false,
		usb_debug: false,
		oem_unlock: false,
	},
	activity: [
		{
			action: "security_patch_synced",
			detail: "2026-09-05",
			timestamp: 1_767_000_000,
		},
		{ action: "keybox_changed", detail: "", timestamp: 1_766_900_000 },
	],
};

let activityClock = 1_767_100_000;

/** Canonical JSON: key order is part of the contract the parsers enforce. */
function keyboxState(): string {
	return JSON.stringify({
		valid: state.keyboxInstalled,
		bundled: !state.keyboxInstalled,
		source: state.keyboxInstalled ? "google_hardware" : "unknown",
		level: state.keyboxInstalled ? "strongbox" : "unknown",
		play_integrity: "not_checked",
		revocation: "not_listed",
	});
}

function soterState(): string {
	return JSON.stringify({ enabled: state.soterSpoof, reboot_required: false });
}

function adbState(): string {
	return JSON.stringify({
		enabled: state.adb.enabled,
		dev_options: state.adb.dev_options,
		usb_debug: state.adb.usb_debug,
		oem_unlock: state.adb.oem_unlock,
	});
}

function activityLog(): string {
	return JSON.stringify(state.activity.slice(0, 30));
}

function recordActivity(action: string, detail: string): string {
	state.activity = [
		{ action, detail, timestamp: activityClock++ },
		...state.activity,
	].slice(0, 30);
	return "ok";
}

interface Response {
	stdout: string;
	code?: number;
	stderr?: string;
	delayMs?: number;
}

function respondToKeymint(args: readonly string[]): Response {
	const [command, ...rest] = args;
	switch (command) {
		case "--webui-get-keybox-state":
			return { stdout: keyboxState() };
		case "--webui-check-keybox-revocation":
			return { stdout: "not_listed" };
		case "--webui-install-keybox":
			state.keyboxInstalled = true;
			return { stdout: "ok" };
		case "--webui-get-tee-status":
			return { stdout: "normal" };
		case "--webui-sync-security-patch": {
			const requested = rest[0] ?? "auto";
			return { stdout: requested === "auto" ? "auto" : requested };
		}
		case "--webui-get-soter-spoof":
			return { stdout: soterState() };
		case "--webui-set-soter-spoof":
			state.soterSpoof = rest[0] === "1";
			return { stdout: soterState() };
		case "--webui-get-adb-disabler":
			return { stdout: adbState() };
		case "--webui-set-adb-disabler":
			state.adb = {
				enabled: rest[0] === "1",
				dev_options: rest[1] === "1",
				usb_debug: rest[2] === "1",
				oem_unlock: rest[3] === "1",
			};
			return { stdout: "adb_disabler_applied" };
		case "--webui-get-activity-log":
			return { stdout: activityLog() };
		case "--webui-clear-activity-log":
			state.activity = [];
			return { stdout: "ok" };
		case "--webui-record-activity":
			return {
				stdout: recordActivity(rest[0] ?? "", decodeBase64(rest[1] ?? "")),
			};
		case "--webui-fetch-security-bulletin":
			return {
				stdout: "<html><body>Security patch level: 2026-09-05</body></html>",
			};
		default:
			return {
				stdout: "",
				code: 1,
				stderr: `unknown keymint command: ${command}`,
			};
	}
}

function respondToInject(args: readonly string[]): Response {
	const [command, ...rest] = args;
	switch (command) {
		case "--webui-get-scoop":
			return { stdout: JSON.stringify(state.scoop) };
		case "--webui-set-scoop": {
			const decoded = JSON.parse(decodeBase64(rest[0] ?? "")) as unknown;
			state.scoop = Array.isArray(decoded) ? (decoded as string[]) : [];
			return { stdout: "ok" };
		}
		case "--webui-get-tee-status":
			return { stdout: "normal" };
		default:
			return {
				stdout: "",
				code: 1,
				stderr: `unknown inject command: ${command}`,
			};
	}
}

function decodeBase64(value: string): string {
	try {
		return atob(value);
	} catch {
		return "";
	}
}

function respondToShell(command: string): Response {
	if (command.includes("ro.product.cpu"))
		return { stdout: `${ABI}\n${ABI}\naarch64\n` };
	if (command.includes("ro.build.version.security_patch"))
		return { stdout: SECURITY_PATCH };
	if (command.startsWith("[ -x ")) return { stdout: "" };
	if (
		command.includes("pm list packages") ||
		command.includes("cmd package list")
	) {
		return {
			stdout: SAMPLE_PACKAGES.map((name) => `package:${name}`).join("\n"),
		};
	}
	if (command.includes("getPackagesInfo")) return { stdout: "" };
	return { stdout: "" };
}

class FakeStdio implements Stdio {
	#listeners: Array<(data: string) => void> = [];

	on(event: "data", listener: (data: string) => void): void {
		if (event === "data") this.#listeners.push(listener);
	}

	emit(): void {
		// The bridge contract exposes emit for internal use only.
	}

	push(chunk: string): void {
		for (const listener of this.#listeners) listener(chunk);
	}
}

class FakeProcess implements ChildProcess {
	readonly stdout = new FakeStdio();
	readonly stderr = new FakeStdio();
	readonly stdin = new FakeStdio();
	#exitListeners: Array<(code: number) => void> = [];
	#errorListeners: Array<(error: Error) => void> = [];

	on(event: "exit", listener: (code: number) => void): void;
	on(event: "error", listener: (error: Error) => void): void;
	on(
		event: "exit" | "error",
		listener: ((code: number) => void) | ((error: Error) => void),
	): void {
		if (event === "exit")
			this.#exitListeners.push(listener as (code: number) => void);
		else this.#errorListeners.push(listener as (error: Error) => void);
	}

	emit(): void {
		// The bridge contract exposes emit for internal use only.
	}

	settle(response: Response): void {
		const delay = response.delayMs ?? 0;
		setTimeout(() => {
			if (response.stdout.length > 0) this.stdout.push(response.stdout);
			if (response.stderr !== undefined) this.stderr.push(response.stderr);
			const code = response.code ?? 0;
			for (const listener of this.#exitListeners) listener(code);
		}, delay);
	}
}

export function createDevBridge(): Bridge {
	return {
		isKsuWebui: () => false,
		exec: (command: string, _options?: ExecOptions): Promise<ExecResult> => {
			const response = respondToShell(command);
			return Promise.resolve({
				errno: response.code ?? 0,
				stdout: response.stdout,
				stderr: response.stderr ?? "",
			});
		},
		spawn: (command: string, args: string[] = []): ChildProcess => {
			const process = new FakeProcess();
			const isInject = command.endsWith("/inject");
			const response = isInject
				? respondToInject(args)
				: respondToKeymint(args);
			process.settle(response);
			return process;
		},
		listPackages: () => Promise.resolve([...SAMPLE_PACKAGES]),
		getPackagesInfo: ((pkg: string | string[]) => {
			const names = Array.isArray(pkg) ? pkg : [pkg];
			const info: PackagesInfo[] = names.map((packageName, index) => ({
				packageName,
				versionName: "1.0.0",
				versionCode: 1,
				appLabel: SAMPLE_LABELS[packageName] ?? packageName,
				isSystem: SAMPLE_SYSTEM_PACKAGES.includes(packageName),
				uid: 10_000 + index,
			}));
			return Promise.resolve(Array.isArray(pkg) ? info : info[0]);
		}) as Bridge["getPackagesInfo"],
	};
}
