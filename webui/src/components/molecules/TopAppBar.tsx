import type { ReactNode } from "react";
import { cx } from "../../utils/cx";

export interface TopAppBarProps {
	title: string;
	onBack?: () => void;
	backLabel?: string;
	actions?: ReactNode;
	className?: string;
}

export function TopAppBar({
	title,
	onBack,
	backLabel = "Back",
	actions,
	className,
}: TopAppBarProps) {
	return (
		<header
			style={{ paddingTop: "var(--omk-top-inset)" }}
			className={cx(
				"sticky top-0 z-10 border-b border-omk-divider bg-omk-surface",
				className,
			)}
		>
			<div
				style={{ height: "var(--omk-appbar-height)" }}
				className="flex items-center gap-2 px-2"
			>
				{onBack ? (
					<button
						type="button"
						onClick={onBack}
						aria-label={backLabel}
						className="inline-flex size-10 shrink-0 items-center justify-center rounded-omk-md text-omk-on active:bg-omk-container-high"
					>
						<svg
							aria-hidden="true"
							focusable="false"
							viewBox="0 0 24 24"
							className="size-6 fill-none stroke-current stroke-2"
						>
							<path
								d="M15 5 8 12l7 7"
								strokeLinecap="round"
								strokeLinejoin="round"
							/>
						</svg>
					</button>
				) : null}
				<h1 className="min-w-0 flex-1 truncate px-2 text-omk-title text-omk-on">
					{title}
				</h1>
				{actions ? (
					<div className="flex shrink-0 items-center gap-1">{actions}</div>
				) : null}
			</div>
		</header>
	);
}
