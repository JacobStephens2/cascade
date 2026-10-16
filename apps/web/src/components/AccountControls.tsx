import { useState } from "react";
import type { AccountSnapshot } from "../core/types";
import type { SyncState } from "../sync/useSync";

/**
 * Optional account controls for syncing listening time across devices. Renders
 * nothing when no sync backend is configured — the app still tracks locally.
 * The core holds the account; this renders its snapshot and sends its commands.
 */
export function AccountControls({
  account,
  sync,
}: {
  account: AccountSnapshot;
  sync: SyncState;
}) {
  const [email, setEmail] = useState("");
  const [showManage, setShowManage] = useState(false);

  if (!sync.available) return null;

  return (
    <div className="account">
      {account.signedInLabel ? (
        <div className="account__signed-in">
          <span className="account__email">{account.signedInLabel}</span>
          <div className="account__row">
            <button
              type="button"
              className="account__link"
              onClick={sync.signOut}
            >
              Sign out
            </button>
            <button
              type="button"
              className="account__link"
              onClick={() => setShowManage((v) => !v)}
            >
              Manage data
            </button>
          </div>
          {showManage && (
            <div className="account__manage">
              <button
                type="button"
                className="account__danger"
                disabled={account.busy}
                onClick={() => {
                  if (confirm("Delete your synced listening data? This can't be undone."))
                    sync.deleteListeningData();
                }}
              >
                Delete listening data
              </button>
              <button
                type="button"
                className="account__danger"
                disabled={account.busy}
                onClick={() => {
                  if (confirm("Delete your account and all synced data? This can't be undone."))
                    sync.deleteAccount();
                }}
              >
                Delete account
              </button>
            </div>
          )}
        </div>
      ) : (
        <form
          className="account__signin"
          onSubmit={(e) => {
            e.preventDefault();
            sync.requestSignInLink(email);
          }}
        >
          <label className="account__caption" htmlFor="account-email">
            Sync across devices
          </label>
          <div className="account__row">
            <input
              id="account-email"
              type="email"
              inputMode="email"
              autoComplete="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="account__input"
            />
            <button
              type="submit"
              className="account__submit"
              disabled={account.busy}
            >
              Email me a link
            </button>
          </div>
        </form>
      )}
      {account.statusLabel && (
        <p className="account__status">{account.statusLabel}</p>
      )}
      {sync.desktopHandoff && (
        <>
          <p className="account__status">
            Opening the Cascade app to finish signing in…
          </p>
          <a className="account__link" href={sync.desktopHandoff}>
            Open the Cascade app to finish signing in
          </a>
        </>
      )}
    </div>
  );
}
