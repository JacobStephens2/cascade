/**
 * Where the cascade-sync-server lives. The base URL comes from `VITE_SYNC_API`
 * at build time; when it's unset the whole account/sync feature is considered
 * unavailable and the UI hides it (the app still tracks listening time
 * locally).
 */
export const SYNC_API_BASE = (import.meta.env.VITE_SYNC_API ?? "").replace(
  /\/$/,
  "",
);

export const syncAvailable = SYNC_API_BASE.length > 0;
