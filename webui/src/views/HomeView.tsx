import IconChevron from "~icons/material-symbols/chevron-right";
import IconDelete from "~icons/material-symbols/delete";
import type { ActivityAction, ActivityEntry } from "../cli";
import { Card } from "../components/atoms/Card";
import { Icon } from "../components/atoms/Icon";
import { IconButton } from "../components/atoms/IconButton";
import { ProgressIndicator } from "../components/atoms/ProgressIndicator";
import { SettingRow, SettingRowList } from "../components/molecules/SettingRow";
import {
	StatusField,
	type StatusTone,
} from "../components/molecules/StatusField";
import type { Identity, KeyboxStatus, TeeStatus } from "../state/identity";
import { tr } from "../utils/tr";

export interface ActivityState {
	status: "loading" | "ready" | "error";
	entries: ActivityEntry[];
}

export interface HomeViewProps {
	identity: Identity;
	activity: ActivityState;
	activityClearBusy: boolean;
	onClearActivity: () => void;
	onOpenTargets: () => void;
}

function keyboxLabel(
	status: KeyboxStatus,
	source: Identity["keyboxSource"],
): string {
	switch (status) {
		case "loading":
			return tr("home_status_loading", "Checking");
		case "invalid":
			return tr("home_keybox_invalid", "Invalid Keybox");
		case "bundled":
			return tr("home_keybox_bundled", "Built-in Keybox");
		case "error":
			return tr("home_status_error", "Needs attention");
		default:
			break;
	}
	switch (source) {
		case "google_hardware":
			return tr("home_keybox_hardware", "Google hardware root certificate");
		case "google_remote":
			return tr("home_keybox_remote", "Google remote root certificate");
		default:
			return tr("home_keybox_unknown", "Unknown source");
	}
}

function keyboxTone(status: KeyboxStatus): StatusTone {
	if (status === "loading") return "pending";
	if (status === "invalid" || status === "error") return "error";
	return "normal";
}

function levelLabel(level: Identity["keyboxLevel"]): string {
	switch (level) {
		case "tee":
			return tr("home_keybox_tee", "TEE");
		case "strongbox":
			return tr("home_keybox_strongbox", "StrongBox");
		default:
			return tr("home_keybox_level_unknown", "Unknown");
	}
}

function revocationLabel(state: Identity["keyboxRevocation"]): {
	label: string;
	tone: StatusTone;
} {
	switch (state) {
		case "checking":
			return {
				label: tr("home_keybox_status_checking", "Checking"),
				tone: "pending",
			};
		case "not_listed":
			return {
				label: tr("home_keybox_revocation_not_revoked", "Not revoked"),
				tone: "normal",
			};
		case "revoked":
			return {
				label: tr("home_keybox_revocation_revoked", "Revoked"),
				tone: "error",
			};
		case "suspended":
			return {
				label: tr("home_keybox_revocation_revoked", "Revoked"),
				tone: "error",
			};
		case "unknown":
			return {
				label: tr("home_keybox_revocation_check_failed", "Unable to check"),
				tone: "unavailable",
			};
		default:
			return {
				label: tr("home_keybox_status_not_checked", "Not checked"),
				tone: "normal",
			};
	}
}

function teeLabel(status: TeeStatus): { label: string; tone: StatusTone } {
	switch (status) {
		case "loading":
			return { label: tr("home_status_loading", "Checking"), tone: "pending" };
		case "error":
			return {
				label: tr("home_status_error", "Needs attention"),
				tone: "error",
			};
		default:
			return { label: tr("home_tee_normal", "Normal"), tone: "normal" };
	}
}

/** Historical records stay readable even when the action was retired. */
function describeActivity(entry: ActivityEntry): {
	title: string;
	detail: string;
} {
	const action: ActivityAction = entry.action;
	switch (action) {
		case "targets_saved":
			return {
				title: tr("prompt_saved_target", "Config saved"),
				detail: tr("home_selected_apps", "%s apps selected", entry.detail),
			};
		case "keybox_changed":
			return {
				title: tr("menu_replace_keybox", "Change Keybox"),
				detail: tr(
					"prompt_keybox_replaced",
					"Keybox was changed and will reload automatically.",
				),
			};
		case "widevine_installed":
			return {
				title: tr("home_legacy_key_provisioning", "Legacy key provisioning"),
				detail: tr(
					"home_legacy_key_provisioning_detail",
					"A legacy key-provisioning activity was recorded.",
				),
			};
		case "security_patch_synced":
			return {
				title: tr("menu_sync_security_patch", "Sync security patch"),
				detail: tr(
					"prompt_security_patch_sync_complete",
					"Security patch synchronization complete for %s. Please reboot the device.",
					entry.detail,
				),
			};
		case "security_patch_restored":
			return {
				title: tr(
					"menu_restore_default_security_patch",
					"Restore default security patch",
				),
				detail: tr(
					"prompt_security_patch_restored_default",
					"Default security patch restored. Please reboot the device.",
				),
			};
		case "soter_spoof_enabled":
			return {
				title: tr("menu_spoof_soter", "Spoof Soter attestation"),
				detail: tr(
					"prompt_soter_spoof_enabled",
					"Soter spoofing enabled. Reboot to apply.",
				),
			};
		case "soter_spoof_disabled":
			return {
				title: tr("menu_spoof_soter", "Spoof Soter attestation"),
				detail: tr("prompt_soter_spoof_disabled", "Soter spoofing disabled."),
			};
		default:
			return {
				title: tr("tools_adb_disabler", "ADB Disabler"),
				detail: entry.detail,
			};
	}
}

function formatTimestamp(seconds: number): string {
	return new Date(seconds * 1000).toLocaleString();
}

export function HomeView({
	identity,
	activity,
	activityClearBusy,
	onClearActivity,
	onOpenTargets,
}: HomeViewProps) {
	const revocation = revocationLabel(identity.keyboxRevocation);
	const tee = teeLabel(identity.teeStatus);
	const entries = activity.entries;

	return (
		<div className="flex flex-col gap-4 p-4">
			<Card className="grid grid-cols-2 gap-x-4 gap-y-5">
				<StatusField
					label={tr("home_keybox", "Keybox")}
					value={keyboxLabel(identity.keyboxStatus, identity.keyboxSource)}
					tone={keyboxTone(identity.keyboxStatus)}
				/>
				<StatusField
					label={tr("home_keybox_security_level", "Security level")}
					value={levelLabel(identity.keyboxLevel)}
				/>
				<StatusField
					label={tr("home_keybox_revocation", "Revocation")}
					value={revocation.label}
					tone={revocation.tone}
				/>
				<StatusField
					label={tr("home_tee_status", "TEE status")}
					value={tee.label}
					tone={tee.tone}
				/>
				<StatusField
					label={tr("home_security_patch", "Security patch")}
					value={identity.securityPatch ?? "—"}
					tone={identity.securityPatch === null ? "unavailable" : "normal"}
				/>
				<StatusField
					label={tr("home_soter_spoof", "Soter spoof")}
					value={
						identity.soterSpoofEnabled === undefined
							? "—"
							: identity.soterSpoofEnabled
								? tr("soter_spoof_enabled", "Enabled")
								: tr("soter_spoof_disabled", "Disabled")
					}
					tone={
						identity.soterSpoofEnabled === undefined ? "unavailable" : "normal"
					}
				/>
			</Card>

			{/*
			 * `home_selected_apps` is a parameterised string ("%s apps
			 * selected"), so it is the row title with the count substituted —
			 * not a label sitting next to a separate value.
			 */}
			<SettingRowList>
				<SettingRow
					title={tr(
						"home_selected_apps",
						"%s apps selected",
						identity.selectedCount,
					)}
					onPress={onOpenTargets}
					trailing={
						<Icon as={IconChevron} size="md" className="text-omk-muted" />
					}
				/>
			</SettingRowList>

			<Card className="flex flex-col">
				<div className="flex items-center justify-between gap-2">
					<h2 className="text-omk-label text-omk-muted">
						{tr("home_recent_activity", "Recent activity")}
					</h2>
					<IconButton
						aria-label={tr("home_activity_clear", "Clear activity")}
						disabled={activityClearBusy || entries.length === 0}
						onClick={onClearActivity}
					>
						{activityClearBusy ? (
							<ProgressIndicator
								aria-label={tr("home_activity_clear", "Clear activity")}
								size={18}
							/>
						) : (
							<Icon as={IconDelete} size="md" />
						)}
					</IconButton>
				</div>

				{activity.status === "loading" ? (
					<p className="py-6 text-center text-omk-caption text-omk-muted">
						{tr("home_status_loading", "Checking")}
					</p>
				) : null}

				{activity.status === "error" ? (
					<p className="py-6 text-center text-omk-caption text-omk-muted">
						{tr("home_activity_load_error", "Unable to load recent activity.")}
					</p>
				) : null}

				{activity.status === "ready" && entries.length === 0 ? (
					<p className="py-6 text-center text-omk-caption text-omk-muted">
						{tr("home_activity_empty", "No recent activity.")}
					</p>
				) : null}

				{entries.length > 0 ? (
					<ul className="m-0 flex list-none flex-col p-0">
						{entries.map((entry) => {
							const described = describeActivity(entry);
							return (
								<li
									key={`${entry.timestamp}-${entry.action}`}
									className="flex flex-col gap-0.5 border-omk-divider border-t py-3 first:border-t-0"
								>
									<span className="text-omk-body-strong">
										{described.title}
									</span>
									<span className="text-omk-caption text-omk-muted">
										{described.detail}
									</span>
									<span className="text-omk-caption text-omk-muted">
										{formatTimestamp(entry.timestamp)}
									</span>
								</li>
							);
						})}
					</ul>
				) : null}
			</Card>
		</div>
	);
}
