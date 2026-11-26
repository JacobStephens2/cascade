export interface SignInLink {
  token: string;
  /** The link was minted for the Windows app (`app=windows`): hand the
   * token off via `cascade://` instead of redeeming it here. */
  windowsHandoff: boolean;
  /**
   * `href` with the token and app hint removed, for `history.replaceState`.
   * Unrelated params survive: this runs on the player URL, which may carry
   * state of its own. (The standalone /auth page owns its whole URL, so it
   * cleans to the bare pathname instead.)
   */
  cleanedHref: string;
}

/**
 * Find the sign-in token a magic link carries. The emailed link carries it
 * in the URL fragment (`/auth#token=...`) so the browser never sends it to
 * the web server; a query-string token is still accepted so links emailed
 * before the switch keep working until they expire. Returns null when the
 * URL carries no token.
 */
export function parseSignInLink(href: string): SignInLink | null {
  const url = new URL(href);
  const fragment = new URLSearchParams(url.hash.replace(/^#/, ""));
  const token = fragment.get("token") || url.searchParams.get("token");
  if (!token) return null;
  const windowsHandoff =
    (fragment.get("app") || url.searchParams.get("app")) === "windows";

  fragment.delete("token");
  fragment.delete("app");
  url.searchParams.delete("token");
  url.searchParams.delete("app");
  const rest = fragment.toString();
  url.hash = rest ? `#${rest}` : "";

  return { token, windowsHandoff, cleanedHref: url.toString() };
}
