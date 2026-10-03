import type { Command, Effect } from "../core/types";
import * as api from "./api";

/** Dispatches a settle; that dispatch carries the settle's own effects. */
type Settle = (outcome: Command) => void;

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
 * The one request carrier. Every dispatch hands it its effects, whichever
 * command produced them (a tick's push, a sign-in's refresh, an account call);
 * it runs each request effect over HTTP and settles it through `settle`, which
 * is itself a dispatch, so a settle's answer comes back here too. Every request
 * but `revokeSession` must be settled, or the core won't start another.
 * Without a sync server there is nothing to carry.
 */
export function carryRequests(
  effects: Effect[],
  settle: Settle,
  keepalive = false,
): void {
  if (!api.syncAvailable) return;
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
