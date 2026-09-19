import { cx } from "../../utils/cx";

export interface DividerProps {
	/** Indents the leading edge to line up with a row's text column. */
	inset?: boolean;
	className?: string;
}

export function Divider({ inset = false, className }: DividerProps) {
	return (
		<hr
			className={cx(
				"m-0 h-px border-0 bg-omk-divider",
				inset && "ms-4",
				className,
			)}
		/>
	);
}
