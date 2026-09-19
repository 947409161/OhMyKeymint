import {
	DialogPanel,
	DialogTitle,
	Dialog as HeadlessDialog,
} from "@headlessui/react";
import type { ReactNode } from "react";
import { cx } from "../../utils/cx";

export interface DialogProps {
	open: boolean;
	onClose: () => void;
	title: string;
	children: ReactNode;
	className?: string;
}

/** Modal: centred panel over a scrim, with a fresh focus trap per opening. */
export function Dialog({
	open,
	onClose,
	title,
	children,
	className,
}: DialogProps) {
	return (
		<HeadlessDialog
			open={open}
			onClose={onClose}
			className="fixed inset-0 z-50"
		>
			<div
				aria-hidden="true"
				className="fixed inset-0 bg-omk-scrim transition-opacity duration-200"
			/>
			<div className="fixed inset-0 flex items-center justify-center p-4">
				<DialogPanel
					className={cx(
						"w-full max-w-90 rounded-omk-lg bg-omk-surface p-4 shadow-none",
						className,
					)}
				>
					<DialogTitle className="text-omk-title text-omk-on">
						{title}
					</DialogTitle>
					<div className="mt-3">{children}</div>
				</DialogPanel>
			</div>
		</HeadlessDialog>
	);
}
