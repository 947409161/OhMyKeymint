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
	className?: string;
}

const ROW = "flex w-full items-start gap-3 px-4 text-start";

function RowText({
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
 * The shared row anatomy: a `button` when it navigates, a `div` when it is
 * inert.
 *
 * A row that owns a toggle is neither. It is the control itself — see
 * `SettingRowSwitch` and `SettingRowCheckbox` — because a `<label>` that
 * wraps a control *and* points at it with `htmlFor` forwards the activation
 * a second time, toggling the value back to where it started.
 */
export function SettingRow({
	onPress,
	disabled = false,
	className,
	...content
}: SettingRowProps) {
	const body = <RowText {...content} disabled={disabled} />;
	const base = cx(ROW, "min-h-14 bg-omk-surface", className);

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
	extends Omit<SettingRowProps, "trailing" | "onPress" | "className"> {
	checked: boolean;
	onChange: (checked: boolean) => void;
	controlId: string;
	className?: string;
}

export function SettingRowSwitch({
	checked,
	onChange,
	controlId,
	...row
}: SettingRowControlProps) {
	return (
		<Switch
			id={controlId}
			checked={checked}
			onChange={onChange}
			disabled={row.disabled}
			className={cx("min-h-14 bg-omk-surface", row.className)}
		>
			<RowText {...row} />
		</Switch>
	);
}

export function SettingRowCheckbox({
	checked,
	onChange,
	controlId,
	...row
}: SettingRowControlProps) {
	return (
		<Checkbox
			id={controlId}
			checked={checked}
			onChange={onChange}
			disabled={row.disabled}
			className={cx("min-h-14 bg-omk-surface", row.className)}
		>
			<RowText {...row} />
		</Checkbox>
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
