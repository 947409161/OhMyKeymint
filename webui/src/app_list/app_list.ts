import type { PackagesInfo } from "../bridge";
import { getBridge } from "../bridge";
import type { Config } from "../config";
import { isValidPackageName } from "../package_name";
import { isDev } from "../utils/dev";

const DEFAULT_VISIBLE_SYSTEM_APPS = [
	"com.google.android.gsf",
	"com.google.android.gms",
	"com.android.vending",
] as const;

const PACKAGE_INFO_BATCH_SIZE = 32;

const PACKAGE_LIST_COMMANDS = [
	{
		all: "/system/bin/pm list packages --user 0",
		userInstalled: "/system/bin/pm list packages -3 --user 0",
	},
	{
		all: "cmd package list packages --user 0",
		userInstalled: "cmd package list packages -3 --user 0",
	},
] as const;

function afterPaint(): Promise<void> {
	return new Promise((resolve) => {
		window.requestAnimationFrame(() => window.setTimeout(resolve, 0));
	});
}

function normalizeSearchQuery(query: string): string {
	return query.trim().toLocaleLowerCase();
}

function parsePackageList(stdout: string): string[] {
	return stdout
		.split(/\r?\n/)
		.map((line) => line.trim())
		.filter((line) => line.startsWith("package:"))
		.map((line) => line.slice("package:".length))
		.filter(isValidPackageName);
}

/**
 * Runs one package-manager query, or reports null when the command did not
 * answer. That is not the same as answering with nothing: a device whose user
 * installed no app of their own really does list none.
 */
async function queryPackageList(command: string): Promise<string[] | null> {
	try {
		const result = await getBridge().exec(command);
		if (result.errno !== 0) return null;
		return [...new Set(parsePackageList(result.stdout))].sort();
	} catch {
		return null;
	}
}

/**
 * The manager answers `{ packageName, error }` for a package its own app list
 * does not hold — which is every package until that list has loaded, and stays
 * that way for the whole process if loading it failed. Caching such an answer
 * would pin the app to "not system, no label" for the life of the WebUI, and
 * no refresh could correct it, so only an answer that describes the package is
 * kept and the rest are asked for again on the next fetch.
 */
function describesPackage(info: PackagesInfo): boolean {
	return (
		typeof info.isSystem === "boolean" ||
		(typeof info.appLabel === "string" && info.appLabel.length > 0)
	);
}

interface InstalledPackages {
	/** Every package installed for the current user, system ones included. */
	packages: string[];
	/** The subset the user installed themselves, or null when unknown. */
	userInstalled: Set<string> | null;
}

async function queryInstalledPackages(): Promise<InstalledPackages> {
	// ksu.listPackages() can retain the package-manager snapshot from the
	// WebView process. Query Android's package manager directly on every fetch
	// so apps installed while the WebUI is open appear without a cold start.
	for (const command of PACKAGE_LIST_COMMANDS) {
		const packages = await queryPackageList(command.all);
		if (packages === null || packages.length === 0) continue;
		const userInstalled = await queryPackageList(command.userInstalled);
		return {
			packages,
			userInstalled: userInstalled === null ? null : new Set(userInstalled),
		};
	}

	// Keep compatibility with older KernelSU/APatch WebUI bridges that do not
	// expose exec but do provide listPackages.
	const packages = await getBridge()
		.listPackages("all")
		.catch(() => []);
	const userInstalled = await getBridge()
		.listPackages("user")
		.catch(() => null);
	return {
		packages,
		userInstalled: userInstalled === null ? null : new Set(userInstalled),
	};
}

export type SelectionFilter = "all" | "selected" | "unselected";

export interface AppEntry {
	packageName: string;
	appName: string;
	isSystem: boolean;
}

export interface SelectableAppEntry extends AppEntry {
	selected: boolean;
}

export interface AppListSnapshot {
	revision: number;
	entries: readonly AppEntry[];
	selectedPackages: readonly string[];
	selectedCount: number;
	isWritable: boolean;
}

export type AppListSubscriber = (snapshot: AppListSnapshot) => void;

export class AppList {
	readonly #config: Config;
	readonly #visibleSystemApps = new Set<string>(DEFAULT_VISIBLE_SYSTEM_APPS);
	readonly #packageInfoCache = new Map<string, PackagesInfo>();
	readonly #subscribers = new Set<AppListSubscriber>();
	#entries: AppEntry[] = [];
	#revision = 0;
	#fetchPromise: Promise<boolean> | null = null;

	constructor(config: Config) {
		this.#config = config;
	}

	get revision(): number {
		return this.#revision;
	}

	get isWritable(): boolean {
		return this.#config.isWritable;
	}

	getSnapshot(): AppListSnapshot {
		const selectedPackages = [...new Set(this.#config.get("target"))];
		return {
			revision: this.#revision,
			entries: [...this.#entries],
			selectedPackages,
			selectedCount: selectedPackages.length,
			isWritable: this.#config.isWritable,
		};
	}

	subscribe(subscriber: AppListSubscriber): () => void {
		this.#subscribers.add(subscriber);
		subscriber(this.getSnapshot());
		return () => this.#subscribers.delete(subscriber);
	}

	async fetch(): Promise<boolean> {
		if (this.#fetchPromise !== null) return this.#fetchPromise;
		const request = this.#fetch();
		this.#fetchPromise = request;
		try {
			return await request;
		} finally {
			if (this.#fetchPromise === request) this.#fetchPromise = null;
		}
	}

	getEntries(): readonly AppEntry[] {
		return [...this.#entries];
	}

	getTargetEntries(
		query = "",
		filter: SelectionFilter = "all",
	): SelectableAppEntry[] {
		const selected = new Set(this.#config.get("target"));
		const normalizedQuery = normalizeSearchQuery(query);
		return this.#entries
			.filter(
				(entry) =>
					!entry.isSystem || this.#visibleSystemApps.has(entry.packageName),
			)
			.map((entry) => ({ ...entry, selected: selected.has(entry.packageName) }))
			.filter((entry) => this.#matches(entry, normalizedQuery, filter))
			.sort((left, right) => this.#compareEntries(left, right));
	}

	getSystemEntries(query = ""): SelectableAppEntry[] {
		const selected = new Set(this.#config.get("target"));
		const normalizedQuery = normalizeSearchQuery(query);
		return this.#entries
			.filter((entry) => entry.isSystem)
			.map((entry) => ({ ...entry, selected: selected.has(entry.packageName) }))
			.filter((entry) => this.#matchesSearch(entry, normalizedQuery))
			.sort((left, right) => this.#compareEntries(left, right));
	}

	getSelectedPackages(): string[] {
		return [...new Set(this.#config.get("target"))];
	}

	getSelectedCount(): number {
		return this.getSelectedPackages().length;
	}

	isSelected(packageName: string): boolean {
		return this.#config.get("target").includes(packageName);
	}

	setSelected(packageName: string, selected: boolean): void {
		if (!isValidPackageName(packageName)) return;
		const targets = new Set(this.#config.get("target"));
		const changed = selected
			? !targets.has(packageName)
			: targets.has(packageName);
		if (!changed) return;

		if (selected) targets.add(packageName);
		else targets.delete(packageName);
		this.#config.set("target", [...targets]);
		this.#emitChange();
	}

	toggleSelected(packageName: string): void {
		this.setSelected(packageName, !this.isSelected(packageName));
	}

	selectAll(): void {
		const targets = new Set(this.#config.get("target"));
		let changed = false;
		for (const entry of this.getTargetEntries()) {
			if (targets.has(entry.packageName)) continue;
			targets.add(entry.packageName);
			changed = true;
		}
		if (!changed) return;
		this.#config.set("target", [...targets]);
		this.#emitChange();
	}

	deselectAll(): void {
		if (this.#config.get("target").length === 0) return;
		this.#config.set("target", []);
		this.#emitChange();
	}

	applySystemAppSelection(checkedApps: readonly string[]): void {
		const installedSystemApps = new Set(
			this.#entries
				.filter((entry) => entry.isSystem)
				.map((entry) => entry.packageName),
		);
		const checked = new Set(
			checkedApps.filter(
				(packageName) =>
					isValidPackageName(packageName) &&
					installedSystemApps.has(packageName),
			),
		);

		this.#visibleSystemApps.clear();
		for (const packageName of DEFAULT_VISIBLE_SYSTEM_APPS) {
			this.#visibleSystemApps.add(packageName);
		}
		for (const packageName of checked) this.#visibleSystemApps.add(packageName);

		const targets = new Set(this.#config.get("target"));
		for (const packageName of installedSystemApps) {
			if (checked.has(packageName)) targets.add(packageName);
			else targets.delete(packageName);
		}
		this.#config.set("target", [...targets]);
		this.#emitChange();
	}

	syncSystemAppsWithConfig(): void {
		const targets = new Set(this.#config.get("target"));
		for (const entry of this.#entries) {
			if (entry.isSystem && targets.has(entry.packageName)) {
				this.#visibleSystemApps.add(entry.packageName);
			}
		}
		this.#emitChange();
	}

	async save(): Promise<void> {
		await this.#config.write();
	}

	async #fetch(): Promise<boolean> {
		if (isDev()) return this.#replaceEntries(this.#getDevEntries());

		// KernelSU package APIs cross a synchronous WebView bridge. Yield before
		// each call so the navigation and progress animations can reach the screen.
		await afterPaint();
		const { packages, userInstalled } = await queryInstalledPackages();
		const installedPackages = new Set(packages);

		for (const packageName of this.#packageInfoCache.keys()) {
			if (!installedPackages.has(packageName))
				this.#packageInfoCache.delete(packageName);
		}

		const missingPackages = packages.filter(
			(packageName) => !this.#packageInfoCache.has(packageName),
		);
		for (
			let offset = 0;
			offset < missingPackages.length;
			offset += PACKAGE_INFO_BATCH_SIZE
		) {
			await afterPaint();
			const batch = missingPackages.slice(
				offset,
				offset + PACKAGE_INFO_BATCH_SIZE,
			);
			try {
				const infos = (await getBridge().getPackagesInfo(
					batch,
				)) as PackagesInfo[];
				for (const info of infos) {
					if (
						isValidPackageName(info.packageName) &&
						installedPackages.has(info.packageName) &&
						describesPackage(info)
					) {
						this.#packageInfoCache.set(info.packageName, info);
					}
				}
			} catch {
				// Package names remain selectable when labels or metadata are unavailable.
			}
		}

		return this.#replaceEntries(
			packages.map((packageName) => {
				const info = this.#packageInfoCache.get(packageName);
				return {
					packageName,
					appName:
						typeof info?.appLabel === "string" && info.appLabel
							? info.appLabel
							: packageName,
					// Android itself sorts system apps from the ones the user
					// installed. The manager's app list answers the same question,
					// but only once it has loaded — before that it calls every
					// package a user app, which lists the whole system on screen.
					isSystem:
						userInstalled === null
							? (info?.isSystem ?? false)
							: !userInstalled.has(packageName),
				};
			}),
		);
	}

	#replaceEntries(entries: AppEntry[]): boolean {
		const previousEntries = new Map(
			this.#entries.map((entry) => [entry.packageName, entry]),
		);
		const changed =
			entries.length !== this.#entries.length ||
			entries.some((entry) => {
				const previous = previousEntries.get(entry.packageName);
				return (
					previous?.appName !== entry.appName ||
					previous.isSystem !== entry.isSystem
				);
			});
		if (!changed) return false;

		this.#entries = entries;
		const installedPackages = new Set(
			entries.map((entry) => entry.packageName),
		);
		for (const packageName of this.#visibleSystemApps) {
			if (
				!DEFAULT_VISIBLE_SYSTEM_APPS.includes(
					packageName as (typeof DEFAULT_VISIBLE_SYSTEM_APPS)[number],
				) &&
				!installedPackages.has(packageName)
			) {
				this.#visibleSystemApps.delete(packageName);
			}
		}
		this.#emitChange();
		return true;
	}

	#matches(
		entry: SelectableAppEntry,
		normalizedQuery: string,
		filter: SelectionFilter,
	): boolean {
		const selectionMatches =
			filter === "all" ||
			(filter === "selected" && entry.selected) ||
			(filter === "unselected" && !entry.selected);
		return selectionMatches && this.#matchesSearch(entry, normalizedQuery);
	}

	#matchesSearch(entry: AppEntry, normalizedQuery: string): boolean {
		if (!normalizedQuery) return true;
		return `${entry.appName}\n${entry.packageName}`
			.toLocaleLowerCase()
			.includes(normalizedQuery);
	}

	#compareEntries(left: SelectableAppEntry, right: SelectableAppEntry): number {
		if (left.selected !== right.selected) return left.selected ? -1 : 1;
		return left.appName.localeCompare(right.appName);
	}

	#emitChange(): void {
		this.#revision++;
		const snapshot = this.getSnapshot();
		for (const subscriber of this.#subscribers) subscriber(snapshot);
	}

	#getDevEntries(): AppEntry[] {
		return [
			{
				packageName: "io.github.vvb2060.keyattestation",
				appName: "Key Attestation",
				isSystem: false,
			},
			{
				packageName: "com.example.app",
				appName: "Example App",
				isSystem: false,
			},
			{
				packageName: "com.example.banking",
				appName: "Banking App",
				isSystem: false,
			},
			{
				packageName: "com.google.android.gms",
				appName: "Google Play services",
				isSystem: true,
			},
			{
				packageName: "com.android.vending",
				appName: "Google Play Store",
				isSystem: true,
			},
			{
				packageName: "com.google.android.gsf",
				appName: "Google Services Framework",
				isSystem: true,
			},
		];
	}
}
