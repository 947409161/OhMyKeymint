import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../utils/cx";

export interface IconButtonProps
	extends ButtonHTMLAttributes<HTMLButtonElement> {
	/** Icon-only controls must always expose an accessible name. */
	"aria-label": string;
}

export function IconButton({
	className,
	children,
	type = "button",
	...rest
}: IconButtonProps) {
	return (
		<button
			{...rest}
			type={type}
			className={cx(
				"inline-flex size-10 shrink-0 items-center justify-center rounded-omk-md text-omk-on transition-colors",
				"hover:bg-omk-container active:bg-omk-container-high",
				"disabled:text-omk-disabled disabled:hover:bg-transparent",
				className,
			)}
		>
			{children}
		</button>
	);
}
