import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { PortraitCrop } from "@/lib/api";
import { cn } from "@/lib/utils";

/** Side of the frame on screen, px: the same as the stored portrait, so what you see is what you get. */
const FRAME = 192;
const ZOOM_MAX = 8;

type PortraitCropperProps = {
  file: File;
  /** Colour label: the frame is outlined with it, as the finished portrait will be. */
  color: string | null;
  busy: boolean;
  onDone: (crop: PortraitCrop) => void;
  onCancel: () => void;
};

type Natural = { w: number; h: number };

/**
 * Frames the part of a picture that becomes the portrait. The square is the portrait itself: drag
 * the picture, zoom with the wheel or the slider. The crop is reported as the centre (fractions of
 * the width and height) and the side (fraction of the shorter side), see `portrait::Crop` in Rust.
 */
export function PortraitCropper({ file, color, busy, onDone, onCancel }: PortraitCropperProps) {
  const { t } = useTranslation();
  const [url, setUrl] = useState<string | null>(null);
  const [natural, setNatural] = useState<Natural | null>(null);
  const [broken, setBroken] = useState(false);
  const [view, setView] = useState({ cx: 0.5, cy: 0.5, zoom: 1 });
  const drag = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const u = URL.createObjectURL(file);
    setUrl(u);
    setNatural(null);
    setBroken(false);
    setView({ cx: 0.5, cy: 0.5, zoom: 1 });
    return () => URL.revokeObjectURL(u);
  }, [file]);

  // Keeps the frame inside the picture.
  const clamp = (v: { cx: number; cy: number; zoom: number }, n: Natural) => {
    const zoom = Math.min(ZOOM_MAX, Math.max(1, v.zoom));
    const half = Math.min(n.w, n.h) / zoom / 2;
    const fit = (c: number, len: number) => Math.min(1 - half / len, Math.max(half / len, c));
    return { zoom, cx: fit(v.cx, n.w), cy: fit(v.cy, n.h) };
  };

  // Screen pixels per source pixel.
  const scale = natural ? (FRAME * view.zoom) / Math.min(natural.w, natural.h) : 1;

  const move = (dx: number, dy: number) => {
    if (!natural) return;
    setView((v) => clamp({ ...v, cx: v.cx - dx / (natural.w * scale), cy: v.cy - dy / (natural.h * scale) }, natural));
  };
  const zoomTo = (zoom: number) => natural && setView((v) => clamp({ ...v, zoom }, natural));

  return (
    <div className="space-y-2">
      <p className="font-display text-xs font-semibold uppercase">{t("portrait.frame")}</p>
      {broken ? (
        <p className="text-xs text-state-mortal-text">{t("errors.portrait.unreadable")}</p>
      ) : (
        <div
          style={{ width: FRAME, height: FRAME, ...(color ? ({ "--mark": color } as React.CSSProperties) : {}) }}
          className={cn(
            "relative mx-auto touch-none cursor-grab overflow-hidden bg-muted select-none active:cursor-grabbing",
            color && "outline-2 -outline-offset-2 outline-(--mark)",
          )}
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            drag.current = { x: e.clientX, y: e.clientY };
          }}
          onPointerMove={(e) => {
            if (!drag.current) return;
            move(e.clientX - drag.current.x, e.clientY - drag.current.y);
            drag.current = { x: e.clientX, y: e.clientY };
          }}
          onPointerUp={() => (drag.current = null)}
          onPointerCancel={() => (drag.current = null)}
          onWheel={(e) => zoomTo(view.zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1))}
        >
          {url && (
            <img
              src={url}
              alt=""
              draggable={false}
              onLoad={(e) => {
                const { naturalWidth: w, naturalHeight: h } = e.currentTarget;
                setNatural({ w, h });
              }}
              onError={() => setBroken(true)}
              // The picture's own size and offset follow from the frame; Tailwind's max-width reset must not squeeze it.
              style={
                natural
                  ? {
                      position: "absolute",
                      maxWidth: "none",
                      width: natural.w * scale,
                      height: natural.h * scale,
                      left: FRAME / 2 - view.cx * natural.w * scale,
                      top: FRAME / 2 - view.cy * natural.h * scale,
                    }
                  : { visibility: "hidden" }
              }
            />
          )}
        </div>
      )}
      {!broken && (
        <input
          type="range"
          min={1}
          max={ZOOM_MAX}
          step={0.01}
          value={view.zoom}
          disabled={!natural}
          aria-label={t("portrait.zoom")}
          onChange={(e) => zoomTo(Number(e.target.value))}
          className="block w-full accent-primary"
        />
      )}
      <div className="flex gap-1.5">
        {!broken && (
          <Button
            type="button"
            size="xs"
            disabled={!natural || busy}
            onClick={() => onDone({ cx: view.cx, cy: view.cy, size: 1 / view.zoom })}
          >
            {t("portrait.done")}
          </Button>
        )}
        <Button type="button" size="xs" variant="outline" disabled={busy} onClick={onCancel}>
          {t("portrait.cancel")}
        </Button>
      </div>
    </div>
  );
}
