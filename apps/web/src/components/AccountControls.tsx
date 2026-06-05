import { useState } from "react";
import type { SyncState } from "../sync/useSync";

/**
 * Optional account controls for syncing listening time across devices. Renders
 * nothing when no sync backend is configured — the app still tracks locally.
 */
export function AccountControls({ sync }: { sync: SyncState }) {
  const [email, setEmail] = useState("");
  const [showManage, setShowManage] = useState(false);

  if (!sync.available) return null;

  return (
    <div className="account">
      {sync.account ? (
        <div className="account__signed-in">
          <span className="account__email">Syncing · {sync.account.email}</span>
          <div className="account__row">
            <button
              type="button"
              className="account__link"
              onClick={() => void sync.signOut()}
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
                disabled={sync.busy}
                onClick={() => {
                  if (confirm("Delete your synced listening data? This can't be undone."))
                    void sync.deleteData();
                }}
              >
                Delete listening data
              </button>
              <button
                type="button"
                className="account__danger"
                disabled={sync.busy}
                onClick={() => {
                  if (confirm("Delete your account and all synced data? This can't be undone."))
                    void sync.deleteAccount();
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
            if (email.trim()) void sync.signIn(email.trim());
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
              disabled={sync.busy || !email.trim()}
            >
              Email me a link
            </button>
          </div>
        </form>
      )}
      {sync.status && <p className="account__status">{sync.status}</p>}
    </div>
  );
}
