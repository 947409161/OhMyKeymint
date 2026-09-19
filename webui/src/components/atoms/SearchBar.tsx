import IconClear from "~icons/material-symbols/close";
import IconSearch from "~icons/material-symbols/search";
import { cx } from "../../utils/cx";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";

export interface SearchBarProps {
	value: string;
	onChange: (value: string) => void;
	placeholder: string;
	clearLabel: string;
	className?: string;
}

export function SearchBar({
	value,
	onChange,
	placeholder,
	clearLabel,
	className,
}: SearchBarProps) {
	return (
		<div
			className={cx(
				"flex h-10 items-center gap-2 rounded-omk-md bg-omk-container-high px-3",
				className,
			)}
		>
			<Icon as={IconSearch} size="md" className="text-omk-muted" />
			<input
				type="search"
				value={value}
				placeholder={placeholder}
				aria-label={placeholder}
				onChange={(event) => onChange(event.target.value)}
				className="min-w-0 flex-1 bg-transparent text-omk-body text-omk-on outline-none placeholder:text-omk-muted"
			/>
			{value !== "" ? (
				<IconButton
					aria-label={clearLabel}
					className="size-8"
					onClick={() => onChange("")}
				>
					<Icon as={IconClear} size="sm" />
				</IconButton>
			) : null}
		</div>
	);
}
