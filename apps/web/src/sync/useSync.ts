import { useCallback, useEffect, useRef, useState } from "react";
import type { Command, Snapshot, SyncReason } from "../core/types";
import type { DispatchOptions } from "../core/useCascade";
import { syncAvailable } from "./config";

type Dispatch = (command: Command, options?: DispatchOptions) => void;

export interface SyncState {
  available: boolean;
  /** cascade:// deep link when handing a sign-in off to the Windows app. */
  desktopHandoff: string | null;
  requestSignInLink: (email: string) => void;
  signOut: () => void;
  deleteListeningData: () => void;
  deleteAccount: () => void;
}

/**
 * The shell's sync triggers. The core holds the account and decides what to
 * send, including the routine threshold push from a tick; every dispatch's
 * requests go through the one carrier. This hook only says when the page
 * *can* talk (launch, leaving) and passes on the user's account intents.
 */
export function useSync(
  snapshot: Snapshot | null,
  dispatch: Dispatch,
): SyncState {
  // A cascade:// deep link when this page is handing a sign-in off to the
  // Windows desktop app (links minted with &app=windows); null otherwise.
  const [desktopHandoff, setDesktopHandoff] = useState<string | null>(null);
  const coreReady = snapshot !== null;

  // Without a sync server the account UI is hidden and there is no one to
  // talk to, so these triggers send nothing to the core.
  const dispatchIfSyncAvailable = useCallback(
    (command: Command) => {
      if (syncAvailable) dispatch(command);
    },
    [dispatch],
  );

  const sync = useCallback(
    (reason: SyncReason, keepalive = false) => {
      if (syncAvailable)
        dispatch({ type: "beginListeningSync", reason }, { keepalive });
    },
    [dispatch],
  );

  // Finish a magic-link sign-in if the URL carries a token, then clean the
  // URL. Waits for the core, which holds the account.
  const linkHandledRef = useRef(false);
  useEffect(() => {
    if (!syncAvailable || !coreReady || linkHandledRef.current) return;
    linkHandledRef.current = true;
    const url = new URL(window.location.href);
    const token = url.searchParams.get("token");
    if (!token) return;

    // Links minted for the Windows app carry &app=windows: hand the token off
    // to the desktop app via the cascade:// protocol instead of verifying here,
    // since the single-use token can only be redeemed once. We surface the deep
    // link for the user to confirm (and attempt it automatically) rather than
    // burning it on the web.
    const isWindowsHandoff = url.searchParams.get("app") === "windows";
    const link = url.toString();
    url.searchParams.delete("token");
    url.searchParams.delete("app");
    window.history.replaceState({}, "", url.toString());

    if (isWindowsHandoff) {
      const deepLink = `cascade://auth?token=${encodeURIComponent(token)}`;
      setDesktopHandoff(deepLink);
      // Best-effort auto-launch; the visible link is the reliable fallback if
      // the browser blocks programmatic protocol navigation.
      window.location.href = deepLink;
      return;
    }

    dispatchIfSyncAvailable({ type: "submitSignInLink", input: link });
  }, [coreReady, dispatchIfSyncAvailable]);

  // On launch, fetch the cross-device total straight away. The core sends
  // nothing while signed out, and answers a fresh sign-in with its own refresh.
  const launchSyncedRef = useRef(false);
  useEffect(() => {
    if (!coreReady || launchSyncedRef.current) return;
    launchSyncedRef.current = true;
    sync("refresh");
  }, [coreReady, sync]);

  // Flush on the way out so a closing tab doesn't strand recent listening.
  useEffect(() => {
    const flush = () => sync("flush", true);
    const onHide = () => {
      if (document.visibilityState === "hidden") flush();
    };
    document.addEventListener("visibilitychange", onHide);
    window.addEventListener("pagehide", flush);
    return () => {
      document.removeEventListener("visibilitychange", onHide);
      window.removeEventListener("pagehide", flush);
    };
  }, [sync]);

  return {
    available: syncAvailable,
    desktopHandoff,
    requestSignInLink: useCallback(
      (email: string) => dispatchIfSyncAvailable({ type: "requestSignInLink", email }),
      [dispatchIfSyncAvailable],
    ),
    signOut: useCallback(() => dispatchIfSyncAvailable({ type: "signOut" }), [dispatchIfSyncAvailable]),
    deleteListeningData: useCallback(
      () =>
        dispatchIfSyncAvailable({
          type: "deleteListeningData",
          newDeviceId: crypto.randomUUID(),
        }),
      [dispatchIfSyncAvailable],
    ),
    deleteAccount: useCallback(
      () =>
        dispatchIfSyncAvailable({
          type: "deleteAccount",
          newDeviceId: crypto.randomUUID(),
        }),
      [dispatchIfSyncAvailable],
    ),
  };
}
