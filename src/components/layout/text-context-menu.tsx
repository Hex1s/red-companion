import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

type Field = HTMLInputElement | HTMLTextAreaElement;

/** What the menu was opened over: a text field (with its selection) or plain selected text. */
type Target =
  | { kind: "field"; element: Field; start: number; end: number; editable: boolean }
  | { kind: "selection"; text: string };

/** Input types that hold text; a checkbox or a number spinner has nothing to cut or paste. */
const TEXT_INPUTS = new Set(["text", "search", "url", "tel", "email", "password", "number", ""]);

function textField(node: EventTarget | null): Field | null {
  const el = (node as HTMLElement | null)?.closest("input, textarea");
  if (!el) return null;
  if (el instanceof HTMLInputElement && !TEXT_INPUTS.has(el.type)) return null;
  return el as Field;
}

/**
 * The app's own context menu for text: cut / copy / paste / select all in fields, copy over
 * selected text. It replaces the web view's menu everywhere; elsewhere a right click shows
 * nothing, and the menus of tree nodes and the like (which handle the click themselves) are left
 * alone. Mounted once, at the root.
 */
export function TextContextMenu() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [point, setPoint] = useState({ x: 0, y: 0 });
  const [target, setTarget] = useState<Target | null>(null);
  // Set by an action, so the field is focused again instead of the invisible anchor.
  const restoreFocus = useRef<Field | null>(null);

  useEffect(() => {
    const onContextMenu = (event: MouseEvent) => {
      // A menu of the app (explorer, cards) took this click already.
      if (event.defaultPrevented) return;
      event.preventDefault();
      const field = textField(event.target);
      if (field) {
        const start = field.selectionStart ?? 0;
        const end = field.selectionEnd ?? 0;
        setTarget({ kind: "field", element: field, start, end, editable: !field.readOnly && !field.disabled });
      } else {
        const text = window.getSelection()?.toString() ?? "";
        if (!text) return;
        setTarget({ kind: "selection", text });
      }
      setPoint({ x: event.clientX, y: event.clientY });
      setOpen(true);
    };
    window.addEventListener("contextmenu", onContextMenu);
    return () => window.removeEventListener("contextmenu", onContextMenu);
  }, []);

  /** Focuses the field again with the selection it had; the commands act on the focused field. */
  const focusField = (field: Field, start: number, end: number) => {
    restoreFocus.current = field;
    field.focus();
    try {
      field.setSelectionRange(start, end);
    } catch {
      // Number fields have no selection range; the whole value stays as it is.
    }
  };

  const run = (action: "cut" | "copy" | "paste" | "all") => {
    if (!target) return;
    if (target.kind === "selection") {
      void navigator.clipboard.writeText(target.text).catch(() => undefined);
      return;
    }
    const { element, start, end } = target;
    focusField(element, start, end);
    if (action === "all") {
      element.select();
    } else if (action === "paste") {
      // `insertText` goes through the editing pipeline, so controlled React inputs see an `input` event.
      void navigator.clipboard
        .readText()
        .then((text) => {
          focusField(element, start, end);
          document.execCommand("insertText", false, text);
        })
        .catch(() => undefined);
    } else {
      document.execCommand(action);
    }
  };

  const hasSelection = target?.kind === "selection" || (target?.kind === "field" && target.start !== target.end);
  const editable = target?.kind === "field" && target.editable;

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <span
          aria-hidden
          className="pointer-events-none fixed size-0"
          style={{ left: point.x, top: point.y }}
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        sideOffset={0}
        onCloseAutoFocus={(e) => {
          // Focus goes back to the field an action used, not to the anchor.
          e.preventDefault();
          restoreFocus.current?.focus();
          restoreFocus.current = null;
        }}
      >
        {target?.kind === "field" && (
          <DropdownMenuItem disabled={!editable || !hasSelection} onSelect={() => run("cut")}>
            {t("textmenu.cut")}
          </DropdownMenuItem>
        )}
        <DropdownMenuItem disabled={!hasSelection} onSelect={() => run("copy")}>
          {t("textmenu.copy")}
        </DropdownMenuItem>
        {target?.kind === "field" && (
          <>
            <DropdownMenuItem disabled={!editable} onSelect={() => run("paste")}>
              {t("textmenu.paste")}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onSelect={() => run("all")}>{t("textmenu.select_all")}</DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
