import type { AppError } from "@/bindings/AppError";
import i18n from "@/i18n";

/** What a Tauri command throws: the serialized `AppError` of the Rust side. */
export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === "object" &&
    e !== null &&
    typeof (e as AppError).code === "string" &&
    typeof (e as AppError).params === "object"
  );
}

/** Text for the person: translated by `code`, with what to do next; never raw developer text. */
function translate(e: AppError): string {
  const key = `errors.${e.code}`;
  const params: Record<string, unknown> = { ...e.params };
  if (e.cause) params.cause = translate(e.cause);
  if (i18n.exists(key)) return i18n.t(key, params);
  return i18n.t("errors.unknown", { code: e.code, details: e.message });
}

/**
 * Message for any thrown value. Usable outside components (hooks, event handlers), hence
 * `i18n.t` and not `useTranslation`.
 */
export function errorMessage(e: unknown): string {
  if (isAppError(e)) return translate(e);
  if (e instanceof Error) return e.message;
  return typeof e === "string" ? e : i18n.t("errors.unknown", { code: "ui", details: String(e) });
}

/** Developer text of an error (English), for the "technical details" of a message. */
export function errorDetails(e: unknown): string {
  if (isAppError(e)) return e.cause ? `${e.message}\n${errorDetails(e.cause)}` : e.message;
  return e instanceof Error ? e.message : String(e);
}
