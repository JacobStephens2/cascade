import { useCallback, useEffect, useRef, useState } from "react";
import type { Command, Effect, Snapshot, SyncReason } from "../core/types";
import * as api from "./api";

const ACCOUNT_KEY = "cascade.account.v1";

export interface Account {
  sessionToken: string;
  email: string;
}

export interface SyncState {
  available: boolean;
  account: Account | null;
  status: string | null;
  busy: boolean;
  /** cascade:// deep link when handing a sign-in off to the Windows app. */
  desktopHandoff: string | null;
  signIn: (email: string) => Promise<void>;
  signOut: () => Promise<void>;
  deleteData: () => Promise<void>;
  deleteAccount: () => Promise<void>;
}

function loadAccount(): Account | null {
  try {
    const raw = localStorage.getItem(ACCOUNT_KEY);
    return raw ? (JSON.parse(raw) as Account) : null;
  } catch {
    return null;
  }
}

/**
 * Owns the optional account + the listening-time sync transport. This hook
 * decides when it *can* talk to the server (signed in, page lifecycle); the
 * core decides whether there is anything to say, and what — via the
 * `pushListening` it answers `beginListeningSync` with.
 */
export function useSync(
  snapshot: Snapshot | null,
  dispatch: (command: Command) => Effect[],
): SyncState {
  const [account, setAccount] = useState<Account | null>(() =>
    api.syncAvailable ? loadAccount() : null,
  );
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // A cascade:// deep link when this page is handing a sign-in off to the
  // Windows desktop app (links minted with &app=windows); null otherwise.
  const [desktopHandoff, setDesktopHandoff] = useState<string | null>(null);

  const persistAccount = useCallback((next: Account | null) => {
    setAccount(next);
    try {
      if (next) localStorage.setItem(ACCOUNT_KEY, JSON.stringify(next));
      else localStorage.removeItem(ACCOUNT_KEY);
    } catch {
      // non-fatal
    }
  }, []);

  // Ask the core whether to sync; if it answers with a push, PUT exactly that
  // and report back. Every push must be settled, or the core won't start
  // another.
  const sync = useCallback(
    async (acct: Account, reason: SyncReason, keepalive = false) => {
      const push = dispatch({ type: "beginListeningSync", reason }).find(
        (e): e is Extract<Effect, { type: "pushListening" }> =>
          e.type === "pushListening",
      );
      if (!push) return;
      try {
        const res = await api.putListening(
          acct.sessionToken,
          push.deviceId,
          push.deviceTotalMs,
          keepalive,
        );
        dispatch({
          type: "listeningSyncSucceeded",
          serverTotalMs: res.serverTotalMs,
        });
      } catch (err) {
        const unauthorized =
          err instanceof api.HttpError && err.status === 401;
        const effects = dispatch({ type: "listeningSyncFailed", unauthorized });
        if (effects.some((e) => e.type === "clearSession")) {
          // Session no longer valid — drop it; tracking continues locally.
          persistAccount(null);
          setStatus("Signed out — sign in again to sync.");
        }
      }
    },
    [dispatch, persistAccount],
  );

  // Complete a magic-link sign-in if the URL carries a token, then clean the URL.
  useEffect(() => {
    if (!api.syncAvailable) return;
    const url = new URL(window.location.href);
    const token = url.searchParams.get("token");
    if (!token) return;

    // Links minted for the Windows app carry &app=windows: hand the token off
    // to the desktop app via the cascade:// protocol instead of verifying here,
    // since the single-use token can only be redeemed once. We surface the deep
    // link for the user to confirm (and attempt it automatically) rather than
    // burning it on the web.
    const isWindowsHandoff = url.searchParams.get("app") === "windows";
    url.searchParams.delete("token");
    url.searchParams.delete("app");
    window.history.replaceState({}, "", url.toString());

    if (isWindowsHandoff) {
      const deepLink = `cascade://auth?token=${encodeURIComponent(token)}`;
      setDesktopHandoff(deepLink);
      setStatus("Opening the Cascade app to finish signing in…");
      // Best-effort auto-launch; the visible link is the reliable fallback if
      // the browser blocks programmatic protocol navigation.
      window.location.href = deepLink;
      return;
    }

    setBusy(true);
    setStatus("Signing in…");
    api
      .verify(token)
      .then((res) => {
        persistAccount({ sessionToken: res.sessionToken, email: res.email });
        setStatus(`Signed in as ${res.email}.`);
      })
      .catch(() => setStatus("That sign-in link was invalid or expired."))
      .finally(() => setBusy(false));
  }, [persistAccount]);

  // On sign-in (or launch with an account), do an immediate sync so the
  // cross-device total shows right away. Waits for the core, whose listening
  // restore — and so its device id — lands in the same commit as the first
  // snapshot.
  const coreReady = snapshot !== null;
  const lastSyncedAccountRef = useRef<string | null>(null);
  useEffect(() => {
    if (!account) {
      lastSyncedAccountRef.current = null;
      return;
    }
    if (!coreReady) return;
    if (lastSyncedAccountRef.current === account.sessionToken) return;
    lastSyncedAccountRef.current = account.sessionToken;
    void sync(account, "refresh");
  }, [account, coreReady, sync]);

  // As listening accrues, offer the core a sync; it sends once enough is
  // unsynced.
  useEffect(() => {
    if (!account) return;
    void sync(account, "threshold");
  }, [account, sync, snapshot?.listening.unsyncedMs]);

  // Flush on the way out so a closing tab doesn't strand recent listening.
  useEffect(() => {
    if (!account) return;
    const flush = () => void sync(account, "flush", true);
    const onHide = () => {
      if (document.visibilityState === "hidden") flush();
    };
    document.addEventListener("visibilitychange", onHide);
    window.addEventListener("pagehide", flush);
    return () => {
      document.removeEventListener("visibilitychange", onHide);
      window.removeEventListener("pagehide", flush);
    };
  }, [account, sync]);

  const signIn = useCallback(async (email: string) => {
    setBusy(true);
    setStatus(null);
    try {
      await api.requestLink(email);
      setStatus(`Check ${email} for a sign-in link.`);
    } catch {
      setStatus("Couldn't send the sign-in link. Try again.");
    } finally {
      setBusy(false);
    }
  }, []);

  const signOut = useCallback(async () => {
    const acct = account;
    persistAccount(null);
    setStatus(null);
    if (acct) {
      try {
        await api.logout(acct.sessionToken);
      } catch {
        // already gone server-side or offline — local sign-out stands
      }
    }
  }, [account, persistAccount]);

  const deleteData = useCallback(async () => {
    if (!account) return;
    setBusy(true);
    try {
      await api.deleteListening(account.sessionToken);
      // The core rotates the device id and zeroes the slot in one persisted
      // write, so a stale offline write can't resurrect the deleted total.
      dispatch({ type: "resetListeningData", newDeviceId: crypto.randomUUID() });
      setStatus("Listening data deleted.");
    } catch {
      setStatus("Couldn't delete listening data. Try again.");
    } finally {
      setBusy(false);
    }
  }, [account, dispatch]);

  const deleteAccount = useCallback(async () => {
    if (!account) return;
    setBusy(true);
    try {
      await api.deleteAccount(account.sessionToken);
      dispatch({ type: "resetListeningData", newDeviceId: crypto.randomUUID() });
      persistAccount(null);
      setStatus("Account deleted.");
    } catch {
      setStatus("Couldn't delete the account. Try again.");
    } finally {
      setBusy(false);
    }
  }, [account, dispatch, persistAccount]);

  return {
    available: api.syncAvailable,
    account,
    status,
    busy,
    desktopHandoff,
    signIn,
    signOut,
    deleteData,
    deleteAccount,
  };
}
