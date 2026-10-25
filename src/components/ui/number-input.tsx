import * as React from "react"
import { Input } from "@/components/ui/input"

/**
 * A whole-number field without the browser's spinner: a text field that takes only digits (and a
 * leading minus when `min` allows negatives). The range is not enforced while typing; forms and
 * `FieldBadge` report values outside `min`..`max`, so the bounds only decide whether "-" is typed.
 */
function NumberInput({
  min,
  max: _max,
  onChange,
  ...props
}: Omit<React.ComponentProps<typeof Input>, "type" | "min" | "max"> & { min?: number | string; max?: number | string }) {
  const negative = min === undefined || Number(min) < 0
  return (
    <Input
      {...props}
      type="text"
      inputMode={negative ? "text" : "numeric"}
      autoComplete="off"
      onChange={(e) => {
        const raw = e.target.value
        const clean = (negative && raw.startsWith("-") ? "-" : "") + raw.replace(/\D/g, "")
        if (clean !== raw) e.target.value = clean
        onChange?.(e)
      }}
    />
  )
}

export { NumberInput }
