import {
	type ChildProcess,
	type ExecOptions,
	type ExecResult,
	exec as ksuExec,
	getPackagesInfo as ksuGetPackagesInfo,
	isKsuWebui as ksuIsKsuWebui,
	listPackages as ksuListPackages,
	spawn as ksuSpawn,
	type PackagesInfo,
} from "kernelsu-alt";

export type PackageFilter = "user" | "system" | "all";

export type { ChildProcess, ExecOptions, ExecResult, PackagesInfo };

/**
 * The single seam between application code and the KernelSU WebView bridge.
 * Production resolves it to kernelsu-alt; tests install a fake through
 * setBridge() so no spec needs a real device.
 */
export interface Bridge {
	isKsuWebui(): boolean;
	exec(command: string, options?: ExecOptions): Promise<ExecResult>;
	spawn(command: string, args?: string[], options?: ExecOptions): ChildProcess;
	listPackages(type?: PackageFilter): Promise<string[]>;
	getPackagesInfo(pkg: string): Promise<PackagesInfo>;
	getPackagesInfo(pkg: string[]): Promise<PackagesInfo[]>;
}

export const ksuBridge: Bridge = {
	isKsuWebui: ksuIsKsuWebui,
	exec: ksuExec,
	spawn: ksuSpawn,
	listPackages: ksuListPackages,
	getPackagesInfo: ksuGetPackagesInfo as Bridge["getPackagesInfo"],
};

let current: Bridge = ksuBridge;

export function getBridge(): Bridge {
	return current;
}

/** Replaces the active bridge. Intended for tests and the dev preview. */
export function setBridge(next: Bridge): void {
	current = next;
}

export function resetBridge(): void {
	current = ksuBridge;
}
