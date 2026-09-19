import { Switch as HeadlessSwitch } from "@headlessui/react";
import { cx } from "../../utils/cx";

export interface SwitchProps {
	checked: boolean;
	onChange: (checked: boolean) => void;
	disabled?: boolean;
	id?: string;
	"aria-label"?: string;
	"aria-labelledby"?: string;
}

/**
 * Off is a hairline-bounded container; on is an accent fill. Shape and fill
 * carry the state, so no hue is needed.
 */
export function Switch({
	checked,
	onChange,
	disabled = false,
	id,
	...aria
}: SwitchProps) {
	return (
		<HeadlessSwitch
			checked={checked}
			onChange={onChange}
			disabled={disabled}
			id={id}
			{...aria}
			className={cx(
				"relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors",
				checked
					? "border-omk-accent bg-omk-accent"
					: "border-omk-divider bg-omk-container-high",
				disabled && "opacity-50",
			)}
		>
			<span
				className={cx(
					"inline-block size-4 rounded-full transition-transform duration-150",
					checked
						? "translate-x-4 bg-omk-on-accent"
						: "translate-x-0.5 bg-omk-on",
				)}
			/>
		</HeadlessSwitch>
	);
}
