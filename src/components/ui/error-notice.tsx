import { cn } from "@/lib/utils";

type ErrorNoticeProps = {
  /** Already translated text; nothing is rendered when empty. */
  message: string | null | undefined;
  className?: string;
};

/** Red error line inside a dialog (errors of a dialog stay in the dialog, see CLAUDE.md rule 10). */
export function ErrorNotice({ message, className }: ErrorNoticeProps) {
  if (!message) return null;
  return (
    <p role="alert" className={cn("text-sm text-destructive", className)}>
      {message}
    </p>
  );
}
