import type { ComponentType, SVGProps } from "react";
import { cx } from "../../utils/cx";
import { Icon } from "../atoms/Icon";

export interface NavigationItem {
	id: string;
	label: string;
	icon: ComponentType<SVGProps<SVGSVGElement>>;
}

export interface NavigationBarProps {
	items: readonly NavigationItem[];
	value: number;
	onChange: (index: number) => void;
	className?: string;
}

/**
 * Selected state is carried by weight plus a filled icon mark above the
 * label — no hue, and no extra indicator bar.
 */
export function NavigationBar({
	items,
	value,
	onChange,
	className,
}: NavigationBarProps) {
	return (
		<nav
			aria-label="Main"
			style={{ paddingBottom: "var(--omk-bottom-inset)" }}
			className={cx(
				"sticky bottom-0 z-10 border-t border-omk-divider bg-omk-surface",
				className,
			)}
		>
			<ul
				className="m-0 flex list-none items-stretch p-0"
				style={{ height: "var(--omk-nav-height)" }}
			>
				{items.map((item, index) => {
					const selected = index === value;
					return (
						<li key={item.id} className="flex-1">
							<button
								type="button"
								onClick={() => onChange(index)}
								aria-current={selected ? "page" : undefined}
								className={cx(
									"flex h-full w-full flex-col items-center justify-center gap-1",
									selected ? "text-omk-on" : "text-omk-muted",
								)}
							>
								<Icon as={item.icon} />
								<span
									className={cx("text-omk-caption", selected && "font-medium")}
								>
									{item.label}
								</span>
								<span
									aria-hidden="true"
									className={cx(
										"h-0.5 w-6 rounded-full",
										selected ? "bg-omk-on" : "bg-transparent",
									)}
								/>
							</button>
						</li>
					);
				})}
			</ul>
		</nav>
	);
}
