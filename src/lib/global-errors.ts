import i18n from "@/i18n";
import { isAppError, errorMessage } from "@/lib/errors";
import { status } from "@/store/status-store";

/** Rejected promises nobody caught go to the status area instead of silently vanishing. */
export function installGlobalErrorHandlers() {
  window.addEventListener("unhandledrejection", (event) => {
    event.preventDefault();
    const text = isAppError(event.reason) ? errorMessage(event.reason) : i18n.t("errors.unhandled");
    status.fail(event.reason, "unhandled", { text });
  });
}
