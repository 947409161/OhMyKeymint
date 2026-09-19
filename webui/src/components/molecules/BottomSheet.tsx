import {
	DialogPanel,
	DialogTitle,
	Dialog as HeadlessDialog,
} from "@headlessui/react";
import type { ReactNode } from "react";
import { cx } from "../../utils/cx";

export interface BottomSheetProps {
	open: boolean;
	onClose: () => void;
	title: string;
	/** Optional controls rendered at the leading edge of the title row. */
	startAction?: ReactNode;
	endAction?: ReactNode;
	children: ReactNode;
	className?: string;
}

/**
 * A dialog anchored to the bottom edge. The drag handle is decorative: the
 * sheet is dismissed by the close gesture, the scrim or Escape, all of which
 * Headless UI already manages, so a drag implementation would add a second,
 * less predictable path to the same outcome.
 */
export function BottomSheet({
	open,
	onClose,
	title,
	startAction,
	endAction,
	children,
	className,
}: BottomSheetProps) {
	return (
		<HeadlessDialog
			open={open}
			onClose={onClose}
			className="fixed inset-0 z-50"
		>
			<div aria-hidden="true" className="fixed inset-0 bg-omk-scrim" />
			<div className="fixed inset-0 flex items-end justify-center">
				<DialogPanel
					className={cx(
						"flex max-h-[85dvh] w-full flex-col rounded-t-omk-lg bg-omk-surface",
						className,
					)}
				>
					<div className="flex justify-center pt-2">
						<span
							aria-hidden="true"
							className="h-1 w-8 rounded-full bg-omk-divider"
						/>
					</div>
					<div className="flex min-h-12 items-center gap-1 px-2 pt-1">
						{startAction}
						<DialogTitle className="min-w-0 flex-1 truncate px-2 text-omk-title text-omk-on">
							{title}
						</DialogTitle>
						{endAction}
					</div>
					<div
						style={{ paddingBottom: "calc(var(--omk-bottom-inset) + 16px)" }}
						className="flex min-h-0 flex-1 flex-col px-4 pt-2"
					>
						{children}
					</div>
				</DialogPanel>
			</div>
		</HeadlessDialog>
	);
}
