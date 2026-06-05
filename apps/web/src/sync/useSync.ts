import { useCallback, useEffect, useRef, useState } from "react";
import type { Command, Snapshot } from "../core/types";
import * as api from "./api";

const ACCOUNT_KEY = "cascade.account.v1";
const DEVICE_KEY = "cascade.device.v1";
// How much unsynced listening time to accumulate before pushing to the server.
// Sync cadence lives in the shell, not the core (the core has no idea whether
// we're online or signed in).
const SYNC_THRESHOLD_MS = 30_000;

export interface Account {
  sessionToken: string;
  email: string;
}

export interface SyncState {
  available: boolean;
  account: Account | null;
  status: string | null;
  busy: boolean;
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

/** Stable opaque per-device id; rotated when the user deletes their data so a
 * stale offline write can't resurrect a deleted G-Counter slot. */
function getDeviceId(): string {
  let id = localStorage.getItem(DEVICE_KEY);
  if (!id) {
    id = crypto.randomUUID();
    localStorage.setItem(DEVICE_KEY, id);
  }
  return id;
}

function rotateDeviceId(): string {
  const id = crypto.randomUUID();
  localStorage.setItem(DEVICE_KEY, id);
  return id;
}

/**
 * Owns the optional account + the listening-time sync loop. The core stays
 * pure: this hook reads `snapshot.listening` (deviceTotalMs / unsyncedMs),
 * decides when to talk to the server, and feeds results back in via
 * `applySyncedTotal`.
 */
export function useSync(
  snapshot: Snapshot | null,
  dispatch: (command: Command) => void,
): SyncState {
  const [account, setAccount] = useState<Account | null>(() =>
    api.syncAvailable ? loadAccount() : null,
  );
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const syncingRef = useRef(false);
  // Latest listening figures, kept in a ref so the sync callback is stable.
  const deviceTotalRef = useRef(0);
  const unsyncedRef = useRef(0);
  deviceTotalRef.current = snapshot?.listening.deviceTotalMs ?? 0;
  unsyncedRef.current = snapshot?.listening.unsyncedMs ?? 0;

  const persistAccount = useCallback((next: Account | null) => {
    setAccount(next);
    try {
      if (next) localStorage.setItem(ACCOUNT_KEY, JSON.stringify(next));
      else localStorage.removeItem(ACCOUNT_KEY);
    } catch {
      // non-fatal
    }
  }, []);

  // Push this device's slot and fold the server aggregate back into the core.
  const sync = useCallback(
    async (acct: Account, keepalive = false) => {
      if (syncingRef.current) return;
      syncingRef.current = true;
      const deviceTotalMs = deviceTotalRef.current;
      try {
        const res = await api.putListening(
          acct.sessionToken,
          getDeviceId(),
          deviceTotalMs,
          keepalive,
        );
        dispatch({
          type: "applySyncedTotal",
          syncedThroughMs: deviceTotalMs,
          serverTotalMs: res.serverTotalMs,
        });
      } catch (err) {
        if (err instanceof api.HttpError && err.status === 401) {
          // Session no longer valid — drop it; tracking continues locally.
          persistAccount(null);
          setStatus("Signed out — please sign in again to sync.");
        }
      } finally {
        syncingRef.current = false;
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
    url.searchParams.delete("token");
    window.history.replaceState({}, "", url.toString());
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

  // On sign-in, do an immediate sync so the cross-device total shows right away.
  const lastSyncedAccountRef = useRef<string | null>(null);
  useEffect(() => {
    if (!account) {
      lastSyncedAccountRef.current = null;
      return;
    }
    if (lastSyncedAccountRef.current === account.sessionToken) return;
    lastSyncedAccountRef.current = account.sessionToken;
    void sync(account);
  }, [account, sync]);

  // Threshold-driven sync: once enough unsynced time accrues, push it.
  useEffect(() => {
    if (!account) return;
    if (unsyncedRef.current >= SYNC_THRESHOLD_MS) {
      void sync(account);
    }
  }, [account, sync, snapshot?.listening.unsyncedMs]);

  // Flush on the way out so a closing tab doesn't strand recent listening.
  useEffect(() => {
    if (!account) return;
    const flush = () => {
      if (unsyncedRef.current > 0) void sync(account, true);
    };
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
      // Rotate the device id and zero the local slot together, so a stale
      // offline write can't resurrect the deleted total.
      rotateDeviceId();
      dispatch({ type: "resetListeningData" });
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
      rotateDeviceId();
      dispatch({ type: "resetListeningData" });
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
    signIn,
    signOut,
    deleteData,
    deleteAccount,
  };
}
