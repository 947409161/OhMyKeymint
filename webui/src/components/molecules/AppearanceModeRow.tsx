import { useState } from "react";
import { cx } from "../../utils/cx";
import { Dialog } from "./Dialog";
import { SettingRow } from "./SettingRow";

export interface AppearanceModeLabels {
	title: string;
	summary: string;
	dialogTitle: string;
	options: Record<string, string>;
}

export interface AppearanceModeRowProps {
	value: string;
	onChange: (mode: string) => void;
	modes: readonly string[];
	labels: AppearanceModeLabels;
	controlId: string;
}

/**
 * Copy-free by design: every visible string arrives through `labels`, so the
 * owning view supplies it from i18n rather than the component hard-coding it.
 * The picker is a dialog rather than a `<select>` because the WebUI is
 * touch-only and a native dropdown renders as a desktop popup inside a
 * WebView.
 */
export function AppearanceModeRow({
	value,
	onChange,
	modes,
	labels,
	controlId,
}: AppearanceModeRowProps) {
	const [open, setOpen] = useState(false);

	return (
		<>
			<SettingRow
				title={labels.title}
				summary={labels.summary}
				onPress={() => setOpen(true)}
				trailing={
					<span
						className="text-omk-body text-omk-muted"
						data-testid={`${controlId}-value`}
					>
						{labels.options[value] ?? value}
					</span>
				}
			/>
			<Dialog
				open={open}
				onClose={() => setOpen(false)}
				title={labels.dialogTitle}
			>
				<ul className="m-0 flex list-none flex-col gap-1 p-0">
					{modes.map((mode) => {
						const selected = mode === value;
						return (
							<li key={mode}>
								<button
									type="button"
									aria-pressed={selected}
									onClick={() => {
										onChange(mode);
										setOpen(false);
									}}
									className={cx(
										"flex h-12 w-full items-center justify-between rounded-omk-md px-4 text-start text-omk-body",
										selected
											? "bg-omk-container-high text-omk-on"
											: "text-omk-on",
									)}
								>
									<span>{labels.options[mode] ?? mode}</span>
									{selected ? (
										<svg
											aria-hidden="true"
											focusable="false"
											viewBox="0 0 16 16"
											className="size-4 fill-none stroke-current stroke-[2.5]"
										>
											<path
												d="M3 8.5 6.2 11.5 13 4.5"
												strokeLinecap="round"
												strokeLinejoin="round"
											/>
										</svg>
									) : null}
								</button>
							</li>
						);
					})}
				</ul>
			</Dialog>
		</>
	);
}
