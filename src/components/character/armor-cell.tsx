import { Hint } from "@/components/ui/hint";

type ArmorCellProps = {
  /** Name of the location, for the tooltip and screen readers. */
  label: string;
  /** Armor SP of that location. */
  value: number;
};

/** The SP of one location. No caption: the cells are stacked like the body (head over torso), the tooltip names it. */
export function ArmorCell({ label, value }: ArmorCellProps) {
  return (
    <Hint label={label}>
      <span aria-label={`${label}: ${value}`} className="num flex h-5 w-full items-center justify-center text-sm leading-none font-bold text-emphasis">
        {value}
      </span>
    </Hint>
  );
}
