import { Checkbox as HeadlessCheckbox } from "@headlessui/react";
import { cx } from "../../utils/cx";

export interface CheckboxProps {
	checked: boolean;
	onChange: (checked: boolean) => void;
	disabled?: boolean;
	id?: string;
	"aria-label"?: string;
	"aria-labelledby"?: string;
	className?: string;
}

export function Checkbox({
	checked,
	onChange,
	disabled = false,
	id,
	className,
	...aria
}: CheckboxProps) {
	return (
		<HeadlessCheckbox
			checked={checked}
			onChange={onChange}
			disabled={disabled}
			id={id}
			{...aria}
			className={cx(
				"inline-flex size-[18px] shrink-0 items-center justify-center rounded-omk-sm border transition-colors",
				checked
					? "border-omk-accent bg-omk-accent"
					: "border-omk-divider bg-transparent",
				disabled && "opacity-50",
				className,
			)}
		>
			{checked ? (
				<svg
					aria-hidden="true"
					viewBox="0 0 16 16"
					className="size-3 fill-none stroke-omk-on-accent stroke-[2.5]"
				>
					<path
						d="M3 8.5 6.2 11.5 13 4.5"
						strokeLinecap="round"
						strokeLinejoin="round"
					/>
				</svg>
			) : null}
		</HeadlessCheckbox>
	);
}
