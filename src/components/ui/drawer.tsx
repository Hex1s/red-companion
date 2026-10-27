import type { ReactNode } from "react";
import { Dialog as DialogPrimitive } from "radix-ui";
import { useTranslation } from "react-i18next";
import { XIcon } from "@/components/ui/icons";
import { cn } from "@/lib/utils";

type DrawerProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  side: "left" | "right";
  /** Read by screen readers only; the panel's own content has its headings. */
  title: string;
  children: ReactNode;
  className?: string;
};

/**
 * A slide-in panel for narrow windows: the explorer and the combat column become drawers there.
 * Built on the Radix dialog, so it traps focus, closes on Esc and on a tap outside.
 */
export function Drawer({ open, onOpenChange, side, title, children, className }: DrawerProps) {
  const { t } = useTranslation();
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-black/60 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0" />
        <DialogPrimitive.Content
          aria-describedby={undefined}
          className={cn(
            "fixed inset-y-0 z-40 flex w-[min(22rem,90vw)] flex-col bg-background pt-[env(safe-area-inset-top)] pb-[env(safe-area-inset-bottom)] outline-none duration-200 data-[state=closed]:animate-out data-[state=open]:animate-in",
            side === "left"
              ? "left-0 border-r pl-[env(safe-area-inset-left)] data-[state=closed]:slide-out-to-left data-[state=open]:slide-in-from-left"
              : "right-0 border-l pr-[env(safe-area-inset-right)] data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right",
            className,
          )}
        >
          <DialogPrimitive.Title className="sr-only">{title}</DialogPrimitive.Title>
          <DialogPrimitive.Close className="absolute top-2 right-2 z-10 flex size-8 items-center justify-center text-muted-foreground hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-hidden touch:size-11">
            <XIcon className="size-4" />
            <span className="sr-only">{t("common.close")}</span>
          </DialogPrimitive.Close>
          {children}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
