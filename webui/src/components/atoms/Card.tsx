import type { HTMLAttributes } from "react";
import { cx } from "../../utils/cx";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
	pressable?: boolean;
}

/** Fill-based elevation: a container step, never a shadow. */
export function Card({ pressable = false, className, ...rest }: CardProps) {
	return (
		<div
			{...rest}
			className={cx(
				"rounded-omk-lg bg-omk-container p-4",
				pressable && "transition-colors active:bg-omk-container-high",
				className,
			)}
		/>
	);
}
