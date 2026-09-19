import type { ReactNode } from "react";
import { cx } from "../../utils/cx";

export type BadgeVariant = "solid" | "outline";

export interface BadgeProps {
	variant?: BadgeVariant;
	children: ReactNode;
	className?: string;
}

const VARIANTS: Record<BadgeVariant, string> = {
	solid: "bg-omk-accent text-omk-on-accent",
	outline: "border border-omk-divider text-omk-muted",
};

export function Badge({
	variant = "outline",
	children,
	className,
}: BadgeProps) {
	return (
		<span
			className={cx(
				"inline-flex h-5 items-center rounded-omk-sm px-1.5 text-omk-caption",
				VARIANTS[variant],
				className,
			)}
		>
			{children}
		</span>
	);
}
