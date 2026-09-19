import * as RadixSlider from "@radix-ui/react-slider";
import { cx } from "../../utils/cx";

export interface SliderProps {
	value: number;
	onChange: (value: number) => void;
	min?: number;
	max?: number;
	step?: number;
	disabled?: boolean;
	"aria-label": string;
	className?: string;
}

export function Slider({
	value,
	onChange,
	min = 0,
	max = 100,
	step = 1,
	disabled = false,
	className,
	...aria
}: SliderProps) {
	return (
		<RadixSlider.Root
			value={[value]}
			onValueChange={([next]) => onChange(next ?? min)}
			min={min}
			max={max}
			step={step}
			disabled={disabled}
			className={cx(
				"relative flex h-5 w-full touch-none select-none items-center",
				disabled && "opacity-50",
				className,
			)}
		>
			<RadixSlider.Track className="relative h-1 grow rounded-full bg-omk-container-high">
				<RadixSlider.Range className="absolute h-full rounded-full bg-omk-accent" />
			</RadixSlider.Track>
			{/* role="slider" lives on the thumb, so the accessible name must too. */}
			<RadixSlider.Thumb
				{...aria}
				className="block size-4 rounded-full border border-omk-divider bg-omk-surface"
			/>
		</RadixSlider.Root>
	);
}
