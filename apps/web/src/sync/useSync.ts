import { useCallback, useEffect, useRef, useState } from "react";
import type { Command, Effect, Snapshot, SyncReason } from "../core/types";
import * as api from "./api";

type Dispatch = (command: Command) => Effect[];

export interface SyncState {
  available: boolean;
  /** cascade:// deep link when handing a sign-in off to the Windows app. */
  desktopHandoff: string | null;
  requestSignInLink: (email: string) => void;
  signOut: () => void;
  deleteListeningData: () => void;
  deleteAccount: () => void;
}

/** Run one request and turn its result into the command that settles it. */
async function settleWith<T>(
  call: () => Promise<T>,
  succeeded: (res: T) => Command,
  failed: (unauthorized: boolean) => Command,
): Promise<Command> {
  try {
    return succeeded(await call());
  } catch (err) {
    return failed(err instanceof api.HttpError && err.status === 401);
  }
}

/**
 * Carry every request effect in `effects` over HTTP and settle it with the
 * core. A settle's own answer is carried too (a sign-in answers with a
 * refresh push). Every request but `revokeSession` must be settled, or the
 * core won't start another.
 */
function carry(effects: Effect[], dispatch: Dispatch, keepalive = false): void {
  const settle = (outcome: Command) => carry(dispatch(outcome), dispatch);
  const accountFailed = (unauthorized: boolean): Command => ({
    type: "accountRequestFailed",
    unauthorized,
  });
  for (const effect of effects) {
    switch (effect.type) {
      case "pushListening":
        void settleWith(
          () =>
            api.putListening(
              effect.sessionToken,
              effect.deviceId,
              effect.deviceTotalMs,
              keepalive,
            ),
          (res) => ({
            type: "listeningSyncSucceeded",
            serverTotalMs: res.serverTotalMs,
          }),
          (unauthorized) => ({ type: "listeningSyncFailed", unauthorized }),
        ).then(settle);
        break;
      case "sendSignInLink":
        void settleWith(
          () => api.requestLink(effect.email),
          () => ({ type: "signInLinkSent" }),
          accountFailed,
        ).then(settle);
        break;
      case "verifySignInToken":
        void settleWith(
          () => api.verify(effect.token),
          (res) => ({
            type: "signInVerified",
            sessionToken: res.sessionToken,
            email: res.email,
          }),
          accountFailed,
        ).then(settle);
        break;
      case "revokeSession":
        // Already gone server-side or offline — local sign-out stands.
        api.logout(effect.sessionToken).catch(() => {});
        break;
      case "deleteServerListening":
        void settleWith(
          () => api.deleteListening(effect.sessionToken),
          () => ({
            type: "listeningDataDeleted",
            newDeviceId: crypto.randomUUID(),
          }),
          accountFailed,
        ).then(settle);
        break;
      case "deleteServerAccount":
        void settleWith(
          () => api.deleteAccount(effect.sessionToken),
          () => ({ type: "accountDeleted", newDeviceId: crypto.randomUUID() }),
          accountFailed,
        ).then(settle);
        break;
    }
  }
}

/**
 * The account and listening-sync transport. The core holds the account and
 * decides what to send; this hook carries the requests it is handed and
 * decides only when it *can* talk (page lifecycle).
 */
export function useSync(
  snapshot: Snapshot | null,
  dispatch: Dispatch,
): SyncState {
  // A cascade:// deep link when this page is handing a sign-in off to the
  // Windows desktop app (links minted with &app=windows); null otherwise.
  const [desktopHandoff, setDesktopHandoff] = useState<string | null>(null);
  const coreReady = snapshot !== null;

  const send = useCallback(
    (command: Command) => {
      if (api.syncAvailable) carry(dispatch(command), dispatch);
    },
    [dispatch],
  );

  const sync = useCallback(
    (reason: SyncReason, keepalive = false) => {
      if (api.syncAvailable)
        carry(dispatch({ type: "beginListeningSync", reason }), dispatch, keepalive);
    },
    [dispatch],
  );

  // Finish a magic-link sign-in if the URL carries a token, then clean the
  // URL. Waits for the core, which holds the account.
  const linkHandledRef = useRef(false);
  useEffect(() => {
    if (!api.syncAvailable || !coreReady || linkHandledRef.current) return;
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

    send({ type: "submitSignInLink", input: link });
  }, [coreReady, send]);

  // On launch, fetch the cross-device total straight away. The core sends
  // nothing while signed out, and answers a fresh sign-in with its own refresh.
  const launchSyncedRef = useRef(false);
  useEffect(() => {
    if (!coreReady || launchSyncedRef.current) return;
    launchSyncedRef.current = true;
    sync("refresh");
  }, [coreReady, sync]);

  // As listening accrues, offer the core a sync; it sends once enough is
  // unsynced.
  const unsyncedMs = snapshot?.listening.unsyncedMs;
  useEffect(() => {
    if (unsyncedMs === undefined) return;
    sync("threshold");
  }, [sync, unsyncedMs]);

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
    available: api.syncAvailable,
    desktopHandoff,
    requestSignInLink: useCallback(
      (email: string) => send({ type: "requestSignInLink", email }),
      [send],
    ),
    signOut: useCallback(() => send({ type: "signOut" }), [send]),
    deleteListeningData: useCallback(
      () => send({ type: "deleteListeningData" }),
      [send],
    ),
    deleteAccount: useCallback(() => send({ type: "deleteAccount" }), [send]),
  };
}
