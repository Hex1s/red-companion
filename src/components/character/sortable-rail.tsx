import type { ReactNode } from "react";
import {
  closestCenter,
  DndContext,
  MouseSensor,
  TouchSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type Modifier,
} from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { cn } from "@/lib/utils";

/** The cards move along the list only, and not past its first and last place. */
const withinList: Modifier = ({ transform, draggingNodeRect, containerNodeRect }) => {
  if (!draggingNodeRect || !containerNodeRect) return { ...transform, x: 0 };
  const min = containerNodeRect.top - draggingNodeRect.top;
  const max = containerNodeRect.bottom - draggingNodeRect.bottom;
  return { ...transform, x: 0, y: Math.min(Math.max(transform.y, min), max) };
};

/** A press on a text field must select text, not pick the card up. */
const TEXT_FIELD = "input, textarea, select, [contenteditable=true]";

/** The card may be grabbed anywhere except a text field; buttons still click (the drag starts after a move). */
const grabbable = ({ nativeEvent }: { nativeEvent: Event }) =>
  !(nativeEvent.target as HTMLElement | null)?.closest(TEXT_FIELD);

class RailMouseSensor extends MouseSensor {
  static activators = [{ eventName: "onMouseDown" as const, handler: grabbable }];
}

class RailTouchSensor extends TouchSensor {
  static activators = [{ eventName: "onTouchStart" as const, handler: grabbable }];
}

type SortableRailProps = {
  /** Ids in the current order. */
  ids: number[];
  /** A card was dropped at `to` (a place in the order). */
  onMove: (id: number, to: number) => void;
  children: ReactNode;
};

/** The rail's cards, draggable by any spot: a mouse drag starts after a few pixels, a touch after a short hold (a quick swipe still scrolls). */
export function SortableRail({ ids, onMove, children }: SortableRailProps) {
  const sensors = useSensors(
    useSensor(RailMouseSensor, { activationConstraint: { distance: 6 } }),
    useSensor(RailTouchSensor, { activationConstraint: { delay: 250, tolerance: 8 } }),
  );

  const end = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const to = ids.indexOf(Number(over.id));
    if (to >= 0) onMove(Number(active.id), to);
  };

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} modifiers={[withinList]} onDragEnd={end}>
      <SortableContext items={ids} strategy={verticalListSortingStrategy}>
        {/* The cards' parent: the drag is held inside it, so the combat bar above is out of reach. */}
        <div className="space-y-3">{children}</div>
      </SortableContext>
    </DndContext>
  );
}

type SortableCardProps = { id: number; children: ReactNode };

/** One draggable slot of the rail; the card inside stays as it is. */
export function SortableCard({ id, children }: SortableCardProps) {
  const { listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id });
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      // Portraits are images: the browser must not start its own image drag.
      className={cn(
        "relative touch-manipulation select-none [&_img]:pointer-events-none",
        // The lifted card gets a backing: the board has gaps between its contact legs that would show the cards underneath.
        "bg-transparent ring-1 ring-transparent transition-[background-color,box-shadow] duration-150",
        isDragging && "z-20 scale-[1.02] cursor-grabbing bg-card shadow-xl ring-border",
      )}
      {...listeners}
    >
      {children}
    </div>
  );
}
