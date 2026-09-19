import { useState } from "react";
import IconBuild from "~icons/material-symbols/build";
import IconChevron from "~icons/material-symbols/chevron-right";
import IconHome from "~icons/material-symbols/home";
import IconSettings from "~icons/material-symbols/settings";
import { Badge } from "../components/atoms/Badge";
import { Button } from "../components/atoms/Button";
import { Card } from "../components/atoms/Card";
import { Checkbox } from "../components/atoms/Checkbox";
import { Divider } from "../components/atoms/Divider";
import { Icon } from "../components/atoms/Icon";
import { IconButton } from "../components/atoms/IconButton";
import { ProgressIndicator } from "../components/atoms/ProgressIndicator";
import { Slider } from "../components/atoms/Slider";
import { Switch } from "../components/atoms/Switch";
import { NavigationBar } from "../components/molecules/NavigationBar";
import { SectionHeader } from "../components/molecules/SectionHeader";
import {
	SettingRow,
	SettingRowCheckbox,
	SettingRowList,
	SettingRowSeparator,
	SettingRowSwitch,
} from "../components/molecules/SettingRow";
import { StatusField } from "../components/molecules/StatusField";
import { TopAppBar } from "../components/molecules/TopAppBar";

/**
 * Local review surface for the design standard. Vite drops this from
 * production builds, so it never reaches template/webroot.
 */
const TYPE_ROLES = [
	["text-omk-display", "28 / 600"],
	["text-omk-title", "20 / 600"],
	["text-omk-body", "16 / 400"],
	["text-omk-body-strong", "16 / 500"],
	["text-omk-label", "14 / 500"],
	["text-omk-caption", "13 / 400"],
	["text-omk-mono", "13 / 400 mono"],
] as const;

const SURFACES = [
	["background", "bg-omk-bg"],
	["surface", "bg-omk-surface"],
	["container", "bg-omk-container"],
	["container-high", "bg-omk-container-high"],
] as const;

const THEME_VALUES = ["light", "dark", "amoled"] as const;
const RADIUS_VALUES = ["soft", "tight"] as const;

export interface GalleryViewProps {
	mode: string;
	onModeChange: (mode: string) => void;
	radius: "soft" | "tight";
	onRadiusChange: (radius: "soft" | "tight") => void;
}

export function GalleryView({
	mode,
	onModeChange,
	radius,
	onRadiusChange,
}: GalleryViewProps) {
	const [nav, setNav] = useState(0);
	const [switched, setSwitched] = useState(true);
	const [checked, setChecked] = useState(false);
	const [slide, setSlide] = useState(64);

	return (
		<div className="min-h-dvh bg-omk-bg pb-8 text-omk-on">
			<TopAppBar
				title="Design gallery"
				actions={
					<IconButton aria-label="Settings">
						<Icon as={IconSettings} />
					</IconButton>
				}
			/>

			<SectionHeader>Controls</SectionHeader>
			<div className="px-4">
				<Card className="flex flex-col gap-3">
					<div className="flex flex-wrap gap-2">
						{THEME_VALUES.map((value) => (
							<Button
								key={value}
								variant={mode === value ? "primary" : "secondary"}
								aria-pressed={mode === value}
								aria-label={`Theme: ${value}`}
								onClick={() => onModeChange(value)}
							>
								{value}
							</Button>
						))}
					</div>
					<Divider />
					<div className="flex flex-wrap items-center gap-2">
						{RADIUS_VALUES.map((value) => (
							<Button
								key={value}
								variant={radius === value ? "primary" : "secondary"}
								aria-pressed={radius === value}
								aria-label={`Radius: ${value}`}
								onClick={() => onRadiusChange(value)}
							>
								{value}
							</Button>
						))}
					</div>
				</Card>
			</div>

			<SectionHeader>Typography</SectionHeader>
			<div className="px-4">
				<Card className="flex flex-col gap-3">
					{TYPE_ROLES.map(([token, meta]) => (
						<div
							key={token}
							className="flex items-baseline justify-between gap-3"
						>
							<span className={token}>The quick brown fox</span>
							<span className="shrink-0 text-omk-mono text-omk-muted">
								{meta}
							</span>
						</div>
					))}
				</Card>
			</div>

			<SectionHeader>Surfaces</SectionHeader>
			<div className="px-4">
				<div className="overflow-hidden rounded-omk-lg border border-omk-divider">
					{SURFACES.map(([name, cls]) => (
						<div
							key={name}
							className={`${cls} flex items-center justify-between px-4 py-3`}
						>
							<span className="text-omk-body-strong">{name}</span>
							<span className="text-omk-mono text-omk-muted">{cls}</span>
						</div>
					))}
				</div>
			</div>

			<SectionHeader>Buttons</SectionHeader>
			<div className="flex flex-col gap-3 px-4">
				<Card className="flex flex-wrap items-center gap-2">
					<Button>Primary</Button>
					<Button variant="secondary">Secondary</Button>
					<Button variant="ghost">Ghost</Button>
				</Card>
				<Card className="flex flex-wrap items-center gap-2">
					<Button busy>Primary</Button>
					<Button variant="secondary" busy>
						Secondary
					</Button>
					<Button disabled aria-label="Primary disabled">
						Disabled
					</Button>
					<Button variant="secondary" disabled aria-label="Secondary disabled">
						Disabled
					</Button>
					<IconButton aria-label="Browse">
						<Icon as={IconHome} />
					</IconButton>
					<IconButton aria-label="Open settings" disabled>
						<Icon as={IconSettings} />
					</IconButton>
				</Card>
				<Card className="flex flex-wrap items-center gap-2">
					<Badge>outline</Badge>
					<Badge variant="solid">solid</Badge>
					<ProgressIndicator aria-label="Loading" />
					<ProgressIndicator
						variant="linear"
						aria-label="Loading"
						className="max-w-32"
					/>
				</Card>
			</div>

			<SectionHeader>Settings rows</SectionHeader>
			<div className="px-4">
				<SettingRowList>
					<SettingRowSwitch
						controlId="g-switch"
						title="Switch row"
						summary="Off is a bounded container, on is an accent fill."
						checked={switched}
						onChange={setSwitched}
					/>
					<SettingRowSeparator />
					<SettingRowCheckbox
						controlId="g-check"
						title="Checkbox row"
						summary="Shape carries the state, not hue."
						checked={checked}
						onChange={setChecked}
					/>
					<SettingRowSeparator />
					<SettingRow
						title="Navigate row"
						summary="Rendered as a real button."
						onPress={() => undefined}
						trailing={
							<Icon as={IconChevron} size="md" className="text-omk-muted" />
						}
					/>
					<SettingRowSeparator />
					<SettingRow
						icon={<Icon as={IconBuild} size="md" />}
						title="Value row"
						onPress={() => undefined}
						trailing={
							<span className="flex items-center gap-1">
								<span className="text-omk-body text-omk-muted">
									google_hardware
								</span>
								<Icon as={IconChevron} size="md" className="text-omk-muted" />
							</span>
						}
					/>
					<SettingRowSeparator />
					<SettingRow
						title="Slider row"
						summary={`${slide}%`}
						trailing={
							<Slider
								aria-label="Interface scale"
								value={slide}
								onChange={setSlide}
								className="w-32"
							/>
						}
					/>
					<SettingRowSeparator />
					<SettingRow
						title="Disabled row"
						summary="Inert, and it says so."
						disabled
						trailing={
							<Switch
								checked={false}
								onChange={() => undefined}
								aria-label="Disabled row"
								disabled
							/>
						}
					/>
				</SettingRowList>
			</div>

			<SectionHeader>Status tones</SectionHeader>
			<div className="px-4">
				<Card className="grid grid-cols-2 gap-4">
					<StatusField label="Keybox" value="Google hardware root" />
					<StatusField label="Revocation" value="Revoked" tone="error" />
					<StatusField label="Attestation" value="Checking" tone="pending" />
					<StatusField
						label="Widevine"
						value="Not installed"
						tone="unavailable"
					/>
				</Card>
			</div>

			<SectionHeader>Raw controls</SectionHeader>
			<div className="flex items-center gap-6 px-4">
				<Switch
					checked={switched}
					onChange={setSwitched}
					aria-label="Raw switch"
				/>
				<Checkbox
					checked={checked}
					onChange={setChecked}
					aria-label="Raw checkbox"
				/>
				<Slider
					aria-label="Raw slider"
					value={slide}
					onChange={setSlide}
					className="max-w-40"
				/>
			</div>

			<SectionHeader>Navigation</SectionHeader>
			<div className="mx-4 overflow-hidden rounded-omk-lg border border-omk-divider">
				<NavigationBar
					value={nav}
					onChange={setNav}
					items={[
						{ id: "home", label: "Home", icon: IconHome },
						{ id: "tools", label: "Tools", icon: IconBuild },
						{ id: "settings", label: "Settings", icon: IconSettings },
					]}
				/>
			</div>
		</div>
	);
}
