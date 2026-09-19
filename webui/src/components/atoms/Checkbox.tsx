import { Checkbox as HeadlessCheckbox } from "@headlessui/react";
import type { ReactNode } from "react";
import { cx } from "../../utils/cx";

export interface CheckboxProps {
	checked: boolean;
	onChange: (checked: boolean) => void;
	disabled?: boolean;
	id?: string;
	/** See {@link SwitchProps.children}: the row and the control are one element. */
	children?: ReactNode;
	className?: string;
	"aria-label"?: string;
	"aria-labelledby"?: string;
}

const BOX =
	"inline-flex size-[18px] shrink-0 items-center justify-center rounded-omk-sm border transition-colors";

function Mark() {
	return (
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
	);
}

export function Checkbox({
	checked,
	onChange,
	disabled = false,
	id,
	children,
	className,
	...aria
}: CheckboxProps) {
	const box = cx(
		BOX,
		checked
			? "border-omk-accent bg-omk-accent"
			: "border-omk-divider bg-transparent",
	);

	return (
		<HeadlessCheckbox
			checked={checked}
			onChange={onChange}
			disabled={disabled}
			id={id}
			{...aria}
			className={cx(
				children === undefined
					? cx(box, "align-middle", disabled && "opacity-50", className)
					: cx(
							"flex w-full items-center gap-3 px-4 text-start",
							disabled && "text-omk-disabled",
							className,
						),
			)}
		>
			{children}
			{children === undefined ? (
				checked ? (
					<Mark />
				) : null
			) : (
				<span className={cx(box, disabled && "opacity-50")}>
					{checked ? <Mark /> : null}
				</span>
			)}
		</HeadlessCheckbox>
	);
}
