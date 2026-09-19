import IconApps from "~icons/material-symbols/apps";
import IconKey from "~icons/material-symbols/key";
import IconRefresh from "~icons/material-symbols/refresh";
import IconReset from "~icons/material-symbols/restart-alt";
import IconSecurity from "~icons/material-symbols/security";
import IconShield from "~icons/material-symbols/shield";
import { Divider } from "../components/atoms/Divider";
import { Icon } from "../components/atoms/Icon";
import { SectionHeader } from "../components/molecules/SectionHeader";
import { SettingRow, SettingRowList } from "../components/molecules/SettingRow";
import { tr } from "../utils/tr";

export interface ToolsViewProps {
	busy: "sync" | "restore" | null;
	onOpenTargets: () => void;
	onInstallKeybox: () => void;
	onSyncPatch: () => void;
	onRestorePatch: () => void;
	onOpenAdbDisabler: () => void;
	onOpenSoter: () => void;
}

export function ToolsView({
	busy,
	onOpenTargets,
	onInstallKeybox,
	onSyncPatch,
	onRestorePatch,
	onOpenAdbDisabler,
	onOpenSoter,
}: ToolsViewProps) {
	return (
		<div className="pb-4">
			<SectionHeader>
				{tr("tools_app_management", "App management")}
			</SectionHeader>
			<div className="px-4">
				<SettingRowList>
					<SettingRow
						icon={<Icon as={IconApps} size="md" />}
						title={tr("app_targets_title", "Add package names")}
						summary={tr(
							"tools_app_targets_desc",
							"Choose which apps use this keybox",
						)}
						onPress={onOpenTargets}
					/>
				</SettingRowList>
			</div>

			<SectionHeader>
				{tr("tools_key_management", "Key management")}
			</SectionHeader>
			<div className="px-4">
				<SettingRowList>
					<SettingRow
						icon={<Icon as={IconKey} size="md" />}
						title={tr("menu_replace_keybox", "Change Keybox")}
						summary={tr(
							"tools_keybox_desc",
							"Install a replacement keybox.xml",
						)}
						onPress={onInstallKeybox}
					/>
					<Divider inset />
					<SettingRow
						icon={<Icon as={IconShield} size="md" />}
						title={tr("tools_soter_spoofing", "Spoof Soter attestation")}
						summary={tr(
							"tools_soter_desc",
							"Present a Soter attestation payload",
						)}
						onPress={onOpenSoter}
					/>
				</SettingRowList>
			</div>

			<SectionHeader>
				{tr("tools_security_patch", "Security patch")}
			</SectionHeader>
			<div className="px-4">
				<SettingRowList>
					<SettingRow
						icon={<Icon as={IconRefresh} size="md" />}
						title={tr("menu_sync_security_patch", "Sync security patch")}
						summary={tr(
							"tools_sync_patch_desc",
							"Download the latest bulletin date",
						)}
						disabled={busy !== null}
						onPress={onSyncPatch}
					/>
					<Divider inset />
					<SettingRow
						icon={<Icon as={IconReset} size="md" />}
						title={tr(
							"menu_restore_default_security_patch",
							"Restore default security patch",
						)}
						summary={tr(
							"tools_restore_patch_desc",
							"Go back to the device value",
						)}
						disabled={busy !== null}
						onPress={onRestorePatch}
					/>
				</SettingRowList>
			</div>

			<SectionHeader>{tr("tools_adb_disabler", "ADB Disabler")}</SectionHeader>
			<div className="px-4">
				<SettingRowList>
					<SettingRow
						icon={<Icon as={IconSecurity} size="md" />}
						title={tr("tools_adb_disabler", "ADB Disabler")}
						summary={tr(
							"tools_adb_disabler_desc",
							"Turn off developer options and ADB",
						)}
						onPress={onOpenAdbDisabler}
					/>
				</SettingRowList>
			</div>
		</div>
	);
}
