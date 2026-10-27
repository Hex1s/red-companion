import type { CSSProperties, ReactNode } from "react";
import brackets from "@/assets/card/brackets.svg?raw";
import connector from "@/assets/card/connector.svg?raw";
import middle from "@/assets/card/middle.svg?raw";
import moduleSvg from "@/assets/card/module.svg?raw";
import traces from "@/assets/card/traces.svg?raw";
import { CARD_ART } from "@/assets/card/geometry";
import { cn } from "@/lib/utils";

/** Width of the module slice on screen, px; every other slice follows at the same scale. */
const MODULE_PX = 116;
const SCALE = MODULE_PX / CARD_ART.moduleW;
/** Height of the board's body (module, middle, traces) on screen, px. */
export const BOARD_BODY_PX = CARD_ART.bodyH * SCALE;
/** The board starts this far below the card's top: the strip above it is for the name. */
export const BOARD_TOP_PX = 16;
const share = (v: string) => parseFloat(v) / 100;
/** Where the portrait window lies inside the board's body, px from its top: the right column lines up with it. */
export const WINDOW_TOP_PX = share(CARD_ART.window.top) * BOARD_BODY_PX;
export const WINDOW_BOTTOM_GAP_PX = BOARD_BODY_PX - WINDOW_TOP_PX - share(CARD_ART.window.height) * BOARD_BODY_PX;
const TRACES_PX = CARD_ART.tracesW * SCALE;
const CONNECTOR_PX = CARD_ART.connectorH * SCALE;

type SliceProps = { svg: string; className?: string; style?: CSSProperties };

function Slice({ svg, className, style }: SliceProps) {
  return <span className={cn("block [&>svg]:size-full", className)} style={style} dangerouslySetInnerHTML={{ __html: svg }} />;
}

/**
 * The board drawn behind a character card: a module with the portrait window on the left, circuit traces
 * on the right (the middle stretches), a contact strip along the bottom. The red parts follow `--mark`
 * of the card. The art is static and ours (src/assets/card), so the inline markup is trusted.
 */
export function CardBoard() {
  return (
    <div
      aria-hidden
      className="card-art pointer-events-none absolute inset-x-px bottom-0 overflow-hidden"
      style={{ top: BOARD_TOP_PX }}
    >
      <div className="flex" style={{ height: BOARD_BODY_PX }}>
        <Slice svg={moduleSvg} className="shrink-0" style={{ width: MODULE_PX }} />
        <Slice svg={middle} className="min-w-0 flex-1" />
        <Slice svg={traces} className="shrink-0" style={{ width: TRACES_PX }} />
      </div>
      <Slice svg={connector} className="absolute inset-x-0 bottom-0" style={{ height: CONNECTOR_PX }} />
    </div>
  );
}

/** Space the contact strip takes at the bottom of the card, px: content keeps clear of it. */
export const BOARD_CONNECTOR_PX = CONNECTOR_PX;

type PortraitWindowProps = {
  children: ReactNode;
  /** Small readouts laid over the portrait (they take clicks themselves, the rest lets them through). */
  overlay?: ReactNode;
};

/** The module-sized box at the card's top-left corner; its children fill the board's portrait window. */
export function PortraitWindow({ children, overlay }: PortraitWindowProps) {
  const w = CARD_ART.window;
  return (
    <div className="relative ml-px shrink-0" style={{ width: MODULE_PX, height: BOARD_BODY_PX }}>
      <div
        className="window-cut absolute"
        style={
          {
            left: w.left,
            top: w.top,
            width: w.width,
            height: w.height,
            "--win-x": w.cutX,
            "--win-y": w.cutY,
          } as CSSProperties
        }
      >
        {children}
      </div>
      {overlay && (
        <div
          className="pointer-events-none absolute"
          style={{ left: w.left, top: w.top, width: w.width, height: w.height }}
        >
          {overlay}
        </div>
      )}
      {/* The brackets sit on the frame and reach over the portrait; clicks go through to it. */}
      <div className="card-art pointer-events-none absolute inset-0">
        <Slice svg={brackets} className="size-full" />
      </div>
    </div>
  );
}
