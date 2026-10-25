import { Component, type ErrorInfo, type ReactNode } from "react";
import { withTranslation, type WithTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { errorDetails } from "@/lib/errors";

type ErrorBoundaryProps = WithTranslation & { children: ReactNode };
type ErrorBoundaryState = { error: unknown; componentStack: string | null };

/** Last line of defence: a render error shows a full-screen panel instead of a blank window. */
class ErrorBoundaryBase extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null, componentStack: null };

  static getDerivedStateFromError(error: unknown): Partial<ErrorBoundaryState> {
    return { error: error ?? "unknown" };
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    console.error(error, info.componentStack);
    this.setState({ componentStack: info.componentStack ?? null });
  }

  render() {
    const { error, componentStack } = this.state;
    const { t, children } = this.props;
    if (error === null) return children;
    const details = [errorDetails(error), componentStack].filter(Boolean).join("\n");
    return (
      <div role="alert" className="flex h-screen items-center justify-center bg-background p-6">
        <div className="cut flex w-full max-w-xl flex-col gap-4 p-6 [--cut:12px] [--cut-border:var(--destructive)]">
          <h1 className="text-destructive-text">{t("crash.title")}</h1>
          <p className="text-sm">{t("crash.hint")}</p>
          <details className="text-xs text-muted-foreground">
            <summary className="cursor-pointer">{t("status.details")}</summary>
            <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap break-words select-text">
              {details}
            </pre>
          </details>
          <div>
            <Button type="button" onClick={() => window.location.reload()}>
              {t("crash.reload")}
            </Button>
          </div>
        </div>
      </div>
    );
  }
}

export const ErrorBoundary = withTranslation()(ErrorBoundaryBase);
