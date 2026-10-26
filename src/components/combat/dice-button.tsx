import { useState } from "react";
import { ChevronDown, Dices } from "@/components/ui/icons";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { FieldBadge } from "@/components/ui/field-badge";
import { Hint } from "@/components/ui/hint";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { parseFaces } from "@/lib/dice";

type DiceButtonProps = {
  notation: string;
  /** Lets the GM change the dice (damage); injuries always roll 2d6. */
  onNotationChange?: (notation: string) => void;
  disabled?: boolean;
  tooltip: string;
  /** `manual` = faces read off physical dice, `null` = roll in the app. */
  onRoll: (notation: string, manual: number[] | null) => void;
};

/**
 * Random roll in one click; the arrow next to it opens the manual entry (the dice may be
 * changed and the faces typed in from physical dice).
 */
export function DiceButton({ notation, onNotationChange, disabled, tooltip, onRoll }: DiceButtonProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [facesText, setFacesText] = useState("");

  const faces = parseFaces(facesText);
  const invalid = faces !== null && faces.some((f) => !Number.isInteger(f) || f < 1);

  return (
    <div className="flex">
      <Hint label={tooltip}>
        <Button
          type="button"
          variant="outline"
          disabled={disabled}
          onClick={() => onRoll(notation, null)}
          className="num"
        >
          <Dices />
          {notation}
        </Button>
      </Hint>
      <Popover open={open} onOpenChange={setOpen}>
        <Hint label={t("damage.dice_manual")}>
          <PopoverTrigger asChild>
            <Button type="button" variant="outline" size="icon" disabled={disabled} aria-label={t("damage.dice_manual")}>
              <ChevronDown />
            </Button>
          </PopoverTrigger>
        </Hint>
        <PopoverContent align="end" className="w-64 space-y-3">
          {onNotationChange && (
            <label className="block space-y-1">
              <span className="block text-xs text-muted-foreground uppercase">{t("damage.notation")}</span>
              <Input value={notation} onChange={(e) => onNotationChange(e.target.value)} className="num" />
            </label>
          )}
          <label className="block space-y-1">
            <span className="block text-xs text-muted-foreground uppercase">{t("damage.manual_faces")}</span>
            <FieldBadge error={invalid ? t("validation.faces") : null}>
              {(shown) => (
                <Input
                  autoFocus
                  value={facesText}
                  placeholder="3 6 6 2 1"
                  aria-invalid={shown}
                  onChange={(e) => setFacesText(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !invalid) {
                      e.preventDefault();
                      onRoll(notation, faces);
                      setOpen(false);
                    }
                  }}
                  className="num"
                />
              )}
            </FieldBadge>
          </label>
          <Button
            type="button"
            size="sm"
            disabled={invalid}
            onClick={() => {
              onRoll(notation, faces);
              setOpen(false);
            }}
          >
            {t("damage.accept")}
          </Button>
        </PopoverContent>
      </Popover>
    </div>
  );
}
