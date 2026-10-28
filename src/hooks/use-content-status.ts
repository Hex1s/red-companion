import { useEffect, useState } from "react";
import { api, hasBackend, type ContentStatus } from "@/lib/api";
import { useContentStore } from "@/store/content-store";

/** What is downloaded, the mode and the packs; `null` until loaded. Loads again after any change. */
export function useContentStatus(): ContentStatus | null {
  const version = useContentStore((s) => s.version);
  const [state, setState] = useState<ContentStatus | null>(null);

  useEffect(() => {
    if (!hasBackend) return;
    let cancelled = false;
    api.contentStatus().then(
      (next) => {
        if (!cancelled) setState(next);
      },
      () => {},
    );
    return () => {
      cancelled = true;
    };
  }, [version]);

  return state;
}
