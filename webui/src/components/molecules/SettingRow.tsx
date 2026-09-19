import type { ReactNode } from "react";
import { cx } from "../../utils/cx";
import { Checkbox } from "../atoms/Checkbox";
import { Divider } from "../atoms/Divider";
import { Switch } from "../atoms/Switch";

export interface SettingRowProps {
	icon?: ReactNode;
	title: string;
	summary?: string;
	trailing?: ReactNode;
	disabled?: boolean;
	/** Present makes the whole row an activatable button. */
	onPress?: () => void;
	/** Wires a labelled control to the row title when the row is a label. */
	htmlFor?: string;
	className?: string;
}

const ROW = "flex w-full items-start gap-3 px-4 text-start";

function RowContent({
	icon,
	title,
	summary,
	trailing,
	disabled = false,
}: SettingRowProps) {
	return (
		<>
			{icon ? (
				<span
					className={cx(
						"mt-3.5 flex size-6 items-center",
						disabled ? "text-omk-disabled" : "text-omk-muted",
					)}
				>
					{icon}
				</span>
			) : null}
			<span className="flex min-w-0 flex-1 flex-col justify-center py-3.5">
				<span
					className={cx(
						"text-omk-body-strong",
						disabled ? "text-omk-disabled" : "text-omk-on",
					)}
				>
					{title}
				</span>
				{summary ? (
					<span
						className={cx(
							"text-omk-caption",
							disabled ? "text-omk-disabled" : "text-omk-muted",
						)}
					>
						{summary}
					</span>
				) : null}
			</span>
			{trailing ? (
				<span className="flex shrink-0 items-center self-center">
					{trailing}
				</span>
			) : null}
		</>
	);
}

/**
 * The shared row anatomy. It renders a `button` when it navigates, a
 * `label` when it owns a control, and a plain `div` when it is inert —
 * never a clickable div.
 *
 * Two constraints learned the hard way:
 * - A wrapping `label` does not name a `role="switch"`/`role="checkbox"`
 *   button in Chromium, so the control carries `aria-label`.
 * - Disabled state uses an explicit token, never `opacity`. Opacity
 *   composites to an unpredictable value and pushed the summary to 2.71:1.
 */
export function SettingRow({
	onPress,
	htmlFor,
	disabled = false,
	className,
	...content
}: SettingRowProps) {
	const body = <RowContent {...content} disabled={disabled} />;
	const base = cx(ROW, "min-h-14 bg-omk-surface", className);

	if (htmlFor !== undefined) {
		return (
			<label
				htmlFor={htmlFor}
				aria-disabled={disabled || undefined}
				className={base}
			>
				{body}
			</label>
		);
	}
	if (onPress !== undefined) {
		return (
			<button
				type="button"
				onClick={onPress}
				disabled={disabled}
				className={cx(base, "transition-colors active:bg-omk-container-high")}
			>
				{body}
			</button>
		);
	}
	return (
		<div aria-disabled={disabled || undefined} className={base}>
			{body}
		</div>
	);
}

export interface SettingRowControlProps
	extends Omit<SettingRowProps, "trailing" | "htmlFor" | "onPress"> {
	checked: boolean;
	onChange: (checked: boolean) => void;
	controlId: string;
}

export function SettingRowSwitch({
	checked,
	onChange,
	controlId,
	...row
}: SettingRowControlProps) {
	return (
		<SettingRow
			{...row}
			htmlFor={controlId}
			trailing={
				<Switch
					id={controlId}
					checked={checked}
					onChange={onChange}
					disabled={row.disabled}
					aria-label={row.title}
				/>
			}
		/>
	);
}

export function SettingRowCheckbox({
	checked,
	onChange,
	controlId,
	...row
}: SettingRowControlProps) {
	return (
		<SettingRow
			{...row}
			htmlFor={controlId}
			trailing={
				<Checkbox
					id={controlId}
					checked={checked}
					onChange={onChange}
					disabled={row.disabled}
					aria-label={row.title}
				/>
			}
		/>
	);
}

export interface SettingRowListProps {
	children: ReactNode;
	className?: string;
}

/** A card-like group whose rows are separated by inset hairlines. */
export function SettingRowList({ children, className }: SettingRowListProps) {
	return (
		<div
			className={cx("overflow-hidden rounded-omk-lg bg-omk-surface", className)}
		>
			{children}
		</div>
	);
}

export function SettingRowSeparator() {
	return <Divider inset />;
}
