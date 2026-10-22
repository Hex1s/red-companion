import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Textarea } from "@/components/ui/textarea";
import { errorMessage } from "@/hooks/use-characters";
import { api } from "@/lib/api";
import { status } from "@/store/status-store";

const AUTOSAVE_DELAY_MS = 700;
const MIN_SAVING_MS = 350;
/** Same limit as the backend (`SESSION_NOTES_MAX`). */
const NOTES_MAX = 100_000;

type SessionNotesProps = {
  sessionId: number;
  sessionName: string;
};

/**
 * Free-form text notes of one session. Mount it with `key={sessionId}`: the text belongs to
 * the session it was loaded for, and leaving the page saves an edit still waiting for its timer.
 */
export function SessionNotes({ sessionId, sessionName }: SessionNotesProps) {
  const { t } = useTranslation();
  // `null` until loaded: typing before that would overwrite the stored notes with the empty text.
  const [text, setText] = useState<string | null>(null);
  const latest = useRef("");
  const pending = useRef(false);
  const timer = useRef<number>(undefined);
  const saveId = `notes:${sessionId}`;

  const save = useRef(async () => {});
  save.current = async () => {
    pending.current = false;
    status.show({ id: saveId, kind: "loading", text: t("status.saving") });
    const started = Date.now();
    try {
      await api.setSessionNotes(sessionId, latest.current);
      const wait = Math.max(0, MIN_SAVING_MS - (Date.now() - started));
      window.setTimeout(() => status.show({ id: saveId, kind: "success", text: t("status.saved") }), wait);
    } catch (e) {
      status.show({
        id: saveId,
        kind: "error",
        text: t("notes.save_error", { error: errorMessage(e) }),
      });
    }
  };

  useEffect(() => {
    let cancelled = false;
    api.sessionNotes(sessionId).then(
      (loaded) => {
        if (cancelled) return;
        latest.current = loaded;
        setText(loaded);
      },
      (e: unknown) => status.error(t("notes.load_error", { error: errorMessage(e) }), saveId),
    );
    return () => {
      cancelled = true;
      window.clearTimeout(timer.current);
      if (pending.current) void save.current();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  const edit = (value: string) => {
    latest.current = value;
    setText(value);
    pending.current = true;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void save.current(), AUTOSAVE_DELAY_MS);
  };

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-baseline gap-3">
        <h2 className="text-2xl font-bold">{t("notes.title")}</h2>
        <span className="text-sm text-muted-foreground">{sessionName}</span>
        <span className="num ml-auto text-xs text-muted-foreground">
          {(text ?? "").length} / {NOTES_MAX}
        </span>
      </div>
      <Textarea
        className="min-h-0 flex-1 resize-none"
        value={text ?? ""}
        disabled={text === null}
        maxLength={NOTES_MAX}
        placeholder={t("notes.placeholder")}
        aria-label={t("notes.title")}
        onChange={(e) => edit(e.target.value)}
      />
    </div>
  );
}
