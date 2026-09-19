import IconTune from "~icons/material-symbols/tune";
import { Divider } from "../components/atoms/Divider";
import { Icon } from "../components/atoms/Icon";
import { Slider } from "../components/atoms/Slider";
import { AppearanceModeRow } from "../components/molecules/AppearanceModeRow";
import { LanguageRow } from "../components/molecules/LanguageRow";
import { SectionHeader } from "../components/molecules/SectionHeader";
import { SettingRow, SettingRowList } from "../components/molecules/SettingRow";
import { i18n } from "../i18n";
import { tr } from "../utils/tr";

export interface SettingsViewProps {
	appearance: string;
	onAppearanceChange: (mode: string) => void;
	scale: number;
	onScaleChange: (value: number) => void;
}

export function SettingsView({
	appearance,
	onAppearanceChange,
	scale,
	onScaleChange,
}: SettingsViewProps) {
	return (
		<div className="pb-4">
			{/* No page-level header: the top app bar already names the page. */}
			<div className="px-4 pt-4">
				<SettingRowList>
					<LanguageRow
						controlId="settings-language"
						value={i18n.preference}
						current={i18n.lang}
						languages={i18n.languages}
						labels={{
							title: tr("settings_language", "Language"),
							dialogTitle: tr("settings_language", "Language"),
							automatic: tr("theme_mode_auto", "Follow system"),
						}}
					/>
					<Divider inset />
					<AppearanceModeRow
						controlId="settings-appearance"
						value={appearance}
						onChange={onAppearanceChange}
						modes={["auto", "light", "dark", "amoled"]}
						labels={{
							title: tr("settings_appearance", "Appearance"),
							dialogTitle: tr("settings_appearance", "Appearance"),
							options: {
								auto: tr("theme_mode_auto", "Follow system"),
								light: tr("theme_mode_light", "Light"),
								dark: tr("theme_mode_dark", "Dark"),
								amoled: tr("theme_mode_amoled", "Pure black"),
							},
						}}
					/>
				</SettingRowList>
			</div>

			<SectionHeader>{tr("settings_interaction", "Interaction")}</SectionHeader>
			<div className="px-4">
				<SettingRowList>
					<SettingRow
						icon={<Icon as={IconTune} size="md" />}
						title={tr("settings_interface_scale", "Interface scale")}
						summary={tr(
							"settings_interface_scale_desc",
							"Adjust the overall display size",
						)}
						trailing={
							<span className="w-32">
								<Slider
									aria-label={tr("settings_interface_scale", "Interface scale")}
									value={scale}
									min={80}
									max={120}
									step={5}
									onChange={onScaleChange}
								/>
							</span>
						}
					/>
				</SettingRowList>
			</div>
		</div>
	);
}
