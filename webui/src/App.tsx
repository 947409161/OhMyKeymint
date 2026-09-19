import { useCallback, useEffect, useRef, useState } from "react";
import IconBuild from "~icons/material-symbols/build";
import IconHome from "~icons/material-symbols/home";
import IconSettings from "~icons/material-symbols/settings";
import { MAX_KEYBOX_XML_BYTES } from "./cli";
import { Button } from "./components/atoms/Button";
import { Dialog } from "./components/molecules/Dialog";
import {
	NavigationBar,
	type NavigationItem,
} from "./components/molecules/NavigationBar";
import {
	SettingRowList,
	SettingRowSwitch,
} from "./components/molecules/SettingRow";
import { SnackbarHost, snackbar } from "./components/molecules/Snackbar";
import { TopAppBar } from "./components/molecules/TopAppBar";
import type { SelectedFile } from "./file_selector/file_selector";
import { MODULE_NAME } from "./module_info";
import { fetchLatestSecurityPatch } from "./security_patch";
import { appList, cli, config, fileSelector, keybind } from "./state/app";
import type { Identity } from "./state/identity";
import { useOverlayHistory } from "./state/useOverlayHistory";
import { tr } from "./utils/tr";
import { FileBrowserSheet } from "./views/FileBrowserSheet";
import { GalleryView } from "./views/GalleryView";
import { type ActivityState, HomeView } from "./views/HomeView";
import { SettingsView } from "./views/SettingsView";
import { TargetsView } from "./views/TargetsView";
import { ToolsView } from "./views/ToolsView";

export const APPEARANCE_MODES = ["auto", "light", "dark", "amoled"] as const;
export type AppearanceMode = (typeof APPEARANCE_MODES)[number];

const THEME_KEY = "omk-appearance";
const SCALE_KEY = "omk-ui-scale";
const PAGE_IDS = ["home", "tools", "settings"] as const;

function prefersDark(): boolean {
	return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function resolveTheme(
	mode: AppearanceMode,
	systemDark: boolean,
): "light" | "dark" | "amoled" {
	if (mode === "auto") return systemDark ? "dark" : "light";
	return mode;
}

function readStored(
	key: string,
	allowed: readonly string[],
	fallback: string,
): string {
	try {
		const stored = window.localStorage.getItem(key);
		if (stored !== null && allowed.includes(stored)) return stored;
	} catch {
		// Storage is unavailable in some WebView configurations.
	}
	return fallback;
}

function persist(key: string, value: string): void {
	try {
		window.localStorage.setItem(key, value);
	} catch {
		// Persisting the preference is best effort.
	}
}

const EMPTY_IDENTITY: Identity = {
	moduleStatus: "loading" as const,
	keyboxStatus: "loading" as const,
	keyboxSource: "unknown" as const,
	keyboxLevel: "unknown" as const,
	keyboxRevocation: "not_checked" as const,
	teeStatus: "loading" as const,
	securityPatch: null,
	soterSpoofEnabled: undefined,
	selectedCount: 0,
};

export function App(): React.JSX.Element {
	const [page, setPage] = useState(0);
	const [identity, setIdentity] = useState(EMPTY_IDENTITY);
	const [activity, setActivity] = useState<ActivityState>({
		status: "loading",
		entries: [],
	});
	const [appearance, setAppearance] = useState(() =>
		readStored(THEME_KEY, APPEARANCE_MODES, "auto"),
	);
	const [scale, setScale] = useState(() => readStored(SCALE_KEY, [], "100"));
	const [systemDark, setSystemDark] = useState<boolean>(prefersDark);
	const [patchBusy, setPatchBusy] = useState<"sync" | "restore" | null>(null);
	const [activityClearBusy, setActivityClearBusy] = useState(false);
	const [keyboxFile, setKeyboxFile] = useState<SelectedFile | null>(null);
	const [keyboxBusy, setKeyboxBusy] = useState(false);
	const [adbOpen, setAdbOpen] = useState(false);
	const [soterOpen, setSoterOpen] = useState(false);
	const [targetsOpen, setTargetsOpen] = useState(false);
	// Modal layers owned by a child view, reported up so Escape closes exactly
	// one layer at a time.
	const [targetsOverlayOpen, setTargetsOverlayOpen] = useState(false);
	const [fileSelectorOpen, setFileSelectorOpen] = useState(false);

	useEffect(() => {
		const query = window.matchMedia("(prefers-color-scheme: dark)");
		const onChange = (event: MediaQueryListEvent): void =>
			setSystemDark(event.matches);
		query.addEventListener("change", onChange);
		return () => query.removeEventListener("change", onChange);
	}, []);

	useEffect(() => {
		document.documentElement.dataset.theme = resolveTheme(
			appearance as AppearanceMode,
			systemDark,
		);
		persist(THEME_KEY, appearance);
	}, [appearance, systemDark]);

	useEffect(() => {
		document.documentElement.style.setProperty(
			"--omk-ui-scale",
			String(Number(scale) / 100),
		);
		persist(SCALE_KEY, scale);
	}, [scale]);

	/*
	 * Overlays join the browser history stack so the Android back gesture
	 * closes the topmost one. The host intercepts back presses and hands them
	 * to the page, which is the only way a native WebView back button can
	 * reach React state.
	 */
	useOverlayHistory([
		{ key: "targets", open: targetsOpen, close: () => setTargetsOpen(false) },
		{
			key: "keybox",
			open: keyboxFile !== null,
			close: () => setKeyboxFile(null),
		},
		{ key: "adb", open: adbOpen, close: () => setAdbOpen(false) },
		{ key: "soter", open: soterOpen, close: () => setSoterOpen(false) },
	]);

	// Read through a ref so the handler is registered once: Keybind has no
	// unsubscribe, and re-registering would stack duplicate handlers.
	const escapeTarget = useRef({ targetsOpen: false, dialogOpen: false });
	escapeTarget.current = {
		targetsOpen,
		dialogOpen:
			keyboxFile !== null ||
			adbOpen ||
			soterOpen ||
			targetsOverlayOpen ||
			fileSelectorOpen,
	};

	useEffect(() => {
		keybind.on("keybind-esc", () => {
			const state = escapeTarget.current;
			// Returning false leaves the event alone so Headless UI can close
			// its own dialog; one Escape must never close two layers.
			if (state.dialogOpen || !state.targetsOpen) return false;
			setTargetsOpen(false);
			return true;
		});
	}, []);

	const refreshIdentity = useCallback(async () => {
		await config.read().catch((error: unknown) => {
			console.error("Unable to load the OMK configuration:", error);
		});
		await appList.fetch().catch((error: unknown) => {
			console.error("Unable to load installed packages:", error);
		});
		const count = appList.getSelectedCount();
		const { identity: next, activity: nextActivity } = await import(
			"./state/identity"
		).then(({ loadIdentity }) => loadIdentity(count));
		setIdentity(next);
		setActivity(nextActivity);
	}, []);

	useEffect(() => {
		void refreshIdentity();
	}, [refreshIdentity]);

	const onAppearanceChange = useCallback((next: string) => {
		if ((APPEARANCE_MODES as readonly string[]).includes(next))
			setAppearance(next);
	}, []);

	const clearActivity = useCallback(async () => {
		setActivityClearBusy(true);
		try {
			await cli.clearActivityLog();
			setActivity({ status: "ready", entries: [] });
			snackbar.show(tr("home_activity_cleared", "Activity cleared."));
		} catch (error) {
			snackbar.show(
				error instanceof Error ? error.message : String(error),
				"error",
			);
		} finally {
			setActivityClearBusy(false);
		}
	}, []);

	/**
	 * Delegates to the ported selector rather than a bare <input type="file">.
	 * It requests every MIME type, because Android document providers
	 * frequently do not report XML correctly; it validates the extension and
	 * size; it handles the picker's cancel path on WebViews that never fire a
	 * change event; and it falls back to the shared-storage browser when the
	 * WebView refuses to open a chooser at all.
	 */
	const chooseKeybox = useCallback(async () => {
		try {
			const selected = await fileSelector.getSystemFileContent("xml");
			if (selected !== null) setKeyboxFile(selected);
		} catch (error) {
			snackbar.show(
				error instanceof Error ? error.message : String(error),
				"error",
			);
		}
	}, []);

	const installKeybox = useCallback(async () => {
		if (keyboxFile === null) return;
		setKeyboxBusy(true);
		try {
			await cli.installKeybox(keyboxFile.contents);
			setKeyboxFile(null);
			snackbar.show(tr("prompt_keybox_replaced", "Keybox replaced."));
			await refreshIdentity();
		} catch (error) {
			snackbar.show(
				error instanceof Error ? error.message : String(error),
				"error",
			);
		} finally {
			setKeyboxBusy(false);
		}
	}, [keyboxFile, refreshIdentity]);

	const runPatchAction = useCallback(
		async (mode: "sync" | "restore") => {
			setPatchBusy(mode);
			try {
				if (mode === "restore") {
					await cli.restoreDefaultSecurityPatch();
					snackbar.show(
						tr(
							"prompt_security_patch_restored_default",
							"Default security patch restored. Please reboot the device.",
						),
					);
				} else {
					const date = await fetchLatestSecurityPatch(() =>
						cli.fetchSecurityBulletin(),
					);
					const applied = await cli.syncSecurityPatch(date);
					snackbar.show(
						tr(
							"prompt_security_patch_sync_complete",
							"Security patch set to %s. Please reboot the device.",
							applied,
						),
					);
				}
				await refreshIdentity();
			} catch (error) {
				snackbar.show(
					error instanceof Error ? error.message : String(error),
					"error",
				);
			} finally {
				setPatchBusy(null);
			}
		},
		[refreshIdentity],
	);

	const navItems: NavigationItem[] = [
		{ id: PAGE_IDS[0], label: tr("nav_home", "Home"), icon: IconHome },
		{ id: PAGE_IDS[1], label: tr("nav_tools", "Tools"), icon: IconBuild },
		{
			id: PAGE_IDS[2],
			label: tr("nav_settings", "Settings"),
			icon: IconSettings,
		},
	];

	if (
		import.meta.env.DEV &&
		new URLSearchParams(window.location.search).has("gallery")
	) {
		return (
			<>
				<GalleryView
					theme={resolveTheme(appearance as AppearanceMode, systemDark)}
					appearance={appearance}
					onAppearanceChange={onAppearanceChange}
				/>
				<SnackbarHost />
			</>
		);
	}

	return (
		<div
			className="flex min-h-dvh flex-col bg-omk-bg text-omk-on"
			data-testid="omk-app"
		>
			{/*
			 * The app bar names the module, not the page: the navigation bar
			 * already marks which page is current, so repeating it here was
			 * redundant and cost the title its width.
			 */}
			<TopAppBar title={MODULE_NAME} />

			<main className="flex-1">
				{page === 0 ? (
					<HomeView
						identity={identity}
						activity={activity}
						activityClearBusy={activityClearBusy}
						onClearActivity={() => void clearActivity()}
						onOpenTargets={() => setTargetsOpen(true)}
					/>
				) : null}
				{page === 1 ? (
					<ToolsView
						busy={patchBusy}
						onOpenTargets={() => setTargetsOpen(true)}
						onInstallKeybox={() => void chooseKeybox()}
						onSyncPatch={() => void runPatchAction("sync")}
						onRestorePatch={() => void runPatchAction("restore")}
						onOpenAdbDisabler={() => setAdbOpen(true)}
						onOpenSoter={() => setSoterOpen(true)}
					/>
				) : null}
				{page === 2 ? (
					<SettingsView
						appearance={appearance}
						onAppearanceChange={onAppearanceChange}
						scale={Number(scale)}
						onScaleChange={(value) => setScale(String(value))}
					/>
				) : null}
			</main>

			<NavigationBar items={navItems} value={page} onChange={setPage} />

			{targetsOpen ? (
				<TargetsView
					onClose={() => setTargetsOpen(false)}
					onSaved={() => void refreshIdentity()}
					onNotify={(message, error) =>
						snackbar.show(message, error === true ? "error" : "normal")
					}
					onOverlayChange={setTargetsOverlayOpen}
				/>
			) : null}

			<FileBrowserSheet onOpenChange={setFileSelectorOpen} />

			<Dialog
				open={keyboxFile !== null}
				onClose={() => (keyboxBusy ? undefined : setKeyboxFile(null))}
				title={tr("replace_keybox_title", "Replace keybox.xml")}
			>
				<p className="text-omk-body text-omk-muted">
					{keyboxFile?.name} · {keyboxFile?.contents.byteLength ?? 0} /{" "}
					{MAX_KEYBOX_XML_BYTES} bytes
				</p>
				<div className="mt-4 flex justify-end gap-2">
					<Button
						variant="secondary"
						disabled={keyboxBusy}
						onClick={() => setKeyboxFile(null)}
					>
						{tr("functional_button_cancel", "Cancel")}
					</Button>
					<Button busy={keyboxBusy} onClick={() => void installKeybox()}>
						{tr("functional_button_replace", "Replace")}
					</Button>
				</div>
			</Dialog>

			<SoterDialog
				open={soterOpen}
				onClose={() => setSoterOpen(false)}
				onChanged={refreshIdentity}
			/>
			<AdbDialog
				open={adbOpen}
				onClose={() => setAdbOpen(false)}
				onApplied={() => {
					setAdbOpen(false);
					void refreshIdentity();
				}}
			/>
			<SnackbarHost />
		</div>
	);
}

interface SoterDialogProps {
	open: boolean;
	onClose: () => void;
	onChanged: () => Promise<void>;
}

function SoterDialog({ open, onClose, onChanged }: SoterDialogProps) {
	const [state, setState] = useState<{
		enabled: boolean;
		reboot_required: boolean;
	} | null>(null);
	const [busy, setBusy] = useState(false);

	useEffect(() => {
		if (!open) return;
		setState(null);
		cli.getSoterSpoofState().then(setState, (error: unknown) => {
			snackbar.show(
				error instanceof Error ? error.message : String(error),
				"error",
			);
			onClose();
		});
	}, [open, onClose]);

	const apply = useCallback(
		async (enabled: boolean) => {
			setBusy(true);
			try {
				setState(await cli.setSoterSpoofEnabled(enabled));
				snackbar.show(
					enabled
						? tr(
								"prompt_soter_spoof_enabled",
								"Soter spoofing enabled. Reboot to apply.",
							)
						: tr("prompt_soter_spoof_disabled", "Soter spoofing disabled."),
				);
				await onChanged();
			} catch (error) {
				snackbar.show(
					error instanceof Error ? error.message : String(error),
					"error",
				);
			} finally {
				setBusy(false);
			}
		},
		[onChanged],
	);

	return (
		<Dialog
			open={open}
			onClose={busy ? () => undefined : onClose}
			title={tr("tools_soter_spoofing", "Spoof Soter attestation")}
		>
			<p className="text-omk-caption text-omk-muted">
				{tr("tools_soter_desc", "Present a Soter attestation payload")}
			</p>
			{state === null ? (
				<p className="py-6 text-center text-omk-caption text-omk-muted">
					{tr("home_status_loading", "Checking")}
				</p>
			) : (
				<>
					<SettingRowList className="mt-3">
						<SettingRowSwitch
							controlId="soter-toggle"
							title={tr("tools_soter_spoofing", "Spoof Soter attestation")}
							summary={
								state.reboot_required
									? tr(
											"soter_spoof_reboot_required",
											"Reboot the device to apply this change.",
										)
									: state.enabled
										? tr("soter_spoof_enabled", "Enabled")
										: tr("soter_spoof_disabled", "Disabled")
							}
							checked={state.enabled}
							onChange={(next) => void apply(next)}
							disabled={busy}
						/>
					</SettingRowList>
					<div className="mt-4 flex justify-end">
						<Button variant="secondary" disabled={busy} onClick={onClose}>
							{tr("functional_button_close", "Close")}
						</Button>
					</div>
				</>
			)}
		</Dialog>
	);
}

interface AdbDialogProps {
	open: boolean;
	onClose: () => void;
	onApplied: () => void;
}

function AdbDialog({ open, onClose, onApplied }: AdbDialogProps) {
	const [state, setState] = useState<{
		enabled: boolean;
		dev_options: boolean;
		usb_debug: boolean;
		oem_unlock: boolean;
	} | null>(null);
	const [busy, setBusy] = useState(false);

	useEffect(() => {
		if (!open) return;
		setState(null);
		cli.getAdbDisabler().then(setState, (error: unknown) => {
			snackbar.show(
				error instanceof Error ? error.message : String(error),
				"error",
			);
			onClose();
		});
	}, [open, onClose]);

	const apply = useCallback(async () => {
		if (state === null) return;
		setBusy(true);
		try {
			await cli.setAdbDisabler(
				state.enabled,
				state.dev_options,
				state.usb_debug,
				state.oem_unlock,
			);
			snackbar.show(tr("prompt_adb_disabler_applied", "ADB Disabler applied."));
			onApplied();
		} catch (error) {
			snackbar.show(
				error instanceof Error ? error.message : String(error),
				"error",
			);
		} finally {
			setBusy(false);
		}
	}, [state, onApplied]);

	const rows = [
		[
			"enabled",
			tr("adb_disabler_enabled", "Enabled"),
			tr("adb_disabler_enabled_desc", "Disable ADB on next boot"),
		],
		[
			"dev_options",
			tr("adb_disabler_dev_options", "Developer options"),
			tr("adb_disabler_dev_options_desc", "Hide the developer options entry"),
		],
		[
			"usb_debug",
			tr("adb_disabler_usb_debug", "USB debugging"),
			tr("adb_disabler_usb_debug_desc", "Turn off USB debugging"),
		],
		[
			"oem_unlock",
			tr("adb_disabler_oem_unlock", "OEM unlocking"),
			tr("adb_disabler_oem_unlock_desc", "Turn off OEM unlocking"),
		],
	] as const;

	return (
		<Dialog
			open={open}
			onClose={busy ? () => undefined : onClose}
			title={tr("tools_adb_disabler", "ADB Disabler")}
		>
			<p className="text-omk-caption text-omk-muted">
				{tr("adb_disabler_desc", "Applied on the next boot.")}
			</p>
			{state === null ? (
				<p className="py-6 text-center text-omk-caption text-omk-muted">
					{tr("home_status_loading", "Checking")}
				</p>
			) : (
				<>
					<SettingRowList className="mt-3">
						{rows.map(([key, title, summary], index) => (
							<div key={key}>
								{index > 0 ? <div className="h-px bg-omk-divider" /> : null}
								<SettingRowSwitch
									controlId={`adb-${key}`}
									title={title}
									summary={summary}
									checked={state[key]}
									disabled={busy || (!state.enabled && key !== "enabled")}
									onChange={(next) => setState({ ...state, [key]: next })}
								/>
							</div>
						))}
					</SettingRowList>
					<div className="mt-4 flex justify-end gap-2">
						<Button variant="secondary" disabled={busy} onClick={onClose}>
							{tr("functional_button_cancel", "Cancel")}
						</Button>
						<Button busy={busy} onClick={() => void apply()}>
							{tr("functional_button_apply", "Apply")}
						</Button>
					</div>
				</>
			)}
		</Dialog>
	);
}
