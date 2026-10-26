import { ImagePlus, X } from "@/components/ui/icons";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Hint } from "@/components/ui/hint";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { PortraitCropper } from "@/components/character/portrait-cropper";
import { api, type PortraitCrop } from "@/lib/api";
import { errorMessage } from "@/lib/errors";
import { MARK_PALETTE } from "@/lib/mark-palette";
import { cn } from "@/lib/utils";
import { status } from "@/store/status-store";

/** What the badge shows: the clone number ("Cyberpsycho 2" → "2"), else the first letter of the name. */
export function markLabel(name: string): string {
  const trimmed = name.trim();
  const number = /\s(\d+)$/.exec(trimmed);
  return number ? number[1] : (Array.from(trimmed)[0] ?? "?").toUpperCase();
}

/** Theme token for text on the colour: dark on a light colour, light on a dark one (WCAG luminance). */
function textOn(color: string): string {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(color.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.35 ? "var(--background)" : "var(--foreground)";
}

type ColorMarkProps = {
  /** `#rrggbb` chosen by the GM; `null` = neutral. */
  color: string | null;
  /** Portrait data URI; `null` = initials. */
  portrait?: string | null;
  name: string;
  className?: string;
  /** Outline the portrait in the colour label (off where a frame is drawn around it anyway). */
  framed?: boolean;
};

/**
 * The character's badge: the portrait (outlined in the colour label) or a square with the initial
 * or clone number. The colour is user data, so it comes in through a CSS variable; the text takes
 * the theme's dark or light token to stay legible.
 */
export function ColorMark({ color, portrait, name, className, framed = true }: ColorMarkProps) {
  const style = color
    ? ({ "--mark": color, "--mark-text": textOn(color) } as React.CSSProperties)
    : undefined;
  if (portrait) {
    return (
      <img
        src={portrait}
        alt=""
        aria-hidden
        draggable={false}
        style={style}
        className={cn(
          "size-6 shrink-0 object-cover",
          framed && color && "outline-2 -outline-offset-2 outline-(--mark)",
          className,
        )}
      />
    );
  }
  return (
    <span
      aria-hidden
      style={style}
      className={cn(
        "num inline-flex size-6 shrink-0 items-center justify-center text-xs font-bold transition-colors duration-300",
        color ? "bg-(--mark) text-(--mark-text)" : "bg-muted text-muted-foreground",
        className,
      )}
    >
      {markLabel(name)}
    </span>
  );
}

export type MarkChange = { color?: string | null; portrait?: string | null };

type MarkPickerProps = {
  color: string | null;
  portrait: string | null;
  name: string;
  /** Called at once for a colour (the card changes as you click), for a portrait when the popover closes. */
  onChange: (change: MarkChange) => void;
  className?: string;
  markClassName?: string;
  framed?: boolean;
};

/** The badge as a button: opens the portrait and colour label settings, committed on close. */
export function MarkPicker({ color, portrait, name, onChange, className, markClassName, framed }: MarkPickerProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [draftColor, setDraftColor] = useState(color);
  const [draftPortrait, setDraftPortrait] = useState(portrait);
  const [busy, setBusy] = useState(false);
  // The picture being framed; the settings give way to the cropper while it is set.
  const [framing, setFraming] = useState<File | null>(null);
  const file = useRef<HTMLInputElement>(null);
  useEffect(() => setDraftColor(color), [color]);
  useEffect(() => setDraftPortrait(portrait), [portrait]);

  const close = (next: boolean) => {
    setOpen(next);
    if (next) return;
    setFraming(null);
    const change: MarkChange = {};
    if (draftPortrait !== portrait) change.portrait = draftPortrait;
    if (Object.keys(change).length > 0) onChange(change);
  };

  const make = async (f: File, crop: PortraitCrop) => {
    setBusy(true);
    try {
      setDraftPortrait(await api.makePortrait(await f.arrayBuffer(), crop));
      setFraming(null);
    } catch (e) {
      status.fail(e, "portrait", { text: errorMessage(e) });
    } finally {
      setBusy(false);
    }
  };

  // A label set before the palette existed (any `#rrggbb`) stays visible as one more swatch.
  const swatches: { id: string; color: string }[] = [...MARK_PALETTE];
  if (draftColor !== null && !MARK_PALETTE.some((p) => p.color === draftColor)) {
    swatches.push({ id: "custom", color: draftColor });
  }

  return (
    <Popover open={open} onOpenChange={close}>
      <Hint label={t("portrait.hint")}>
        <PopoverTrigger asChild>
          <button
            type="button"
            aria-label={t("portrait.label")}
            className={cn("shrink-0 outline-none focus-visible:ring-1 focus-visible:ring-ring", className)}
          >
            <ColorMark color={draftColor} portrait={draftPortrait} name={name} className={markClassName} framed={framed} />
          </button>
        </PopoverTrigger>
      </Hint>
      <PopoverContent align="start" className="w-60 space-y-3">
        {framing && (
          <PortraitCropper
            file={framing}
            color={draftColor}
            busy={busy}
            onDone={(crop) => void make(framing, crop)}
            onCancel={() => setFraming(null)}
          />
        )}
        <div className={cn("space-y-1.5", framing && "hidden")}>
          <p className="font-display text-xs font-semibold uppercase">{t("portrait.label")}</p>
          <input
            ref={file}
            type="file"
            accept="image/png,image/jpeg,image/gif,image/bmp,image/webp"
            hidden
            onChange={(e) => {
              setFraming(e.target.files?.[0] ?? null);
              // The same file can be picked again after a cancel.
              e.target.value = "";
            }}
          />
          <div className="flex gap-1.5">
            <Button type="button" size="xs" variant="outline" onClick={() => file.current?.click()}>
              <ImagePlus />
              {draftPortrait ? t("portrait.change") : t("portrait.choose")}
            </Button>
            {draftPortrait && (
              <Button type="button" size="xs" variant="outline" onClick={() => setDraftPortrait(null)}>
                <X />
                {t("portrait.remove")}
              </Button>
            )}
          </div>
        </div>
        <div className={cn("space-y-1.5", framing && "hidden")}>
          <p className="font-display text-xs font-semibold uppercase">{t("color.label")}</p>
          <div className="grid grid-cols-8 gap-1">
            {swatches.map(({ id, color: value }) => (
              <Hint key={value} label={id === "custom" ? t("color.custom") : t(`color.names.${id}`)}>
                <button
                  type="button"
                  aria-label={id === "custom" ? t("color.custom") : t(`color.names.${id}`)}
                  aria-pressed={draftColor === value}
                  onClick={() => {
                    setDraftColor(value);
                    onChange({ color: value });
                  }}
                  style={{ background: value }}
                  className={cn(
                    "aspect-square w-full outline-offset-2 outline-foreground hover:outline-1",
                    draftColor === value && "outline-2",
                  )}
                />
              </Hint>
            ))}
          </div>
          <Button
            type="button"
            size="xs"
            variant="outline"
            disabled={draftColor === null}
            onClick={() => {
              setDraftColor(null);
              onChange({ color: null });
            }}
          >
            {t("color.reset")}
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}
