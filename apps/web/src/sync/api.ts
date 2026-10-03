/**
 * Thin client for the cascade-sync-server. The base URL comes from
 * `VITE_SYNC_API` at build time; when it's unset the whole account/sync feature
 * is considered unavailable and the UI hides it (the app still tracks listening
 * time locally).
 */

const API_BASE = (import.meta.env.VITE_SYNC_API ?? "").replace(/\/$/, "");

export const syncAvailable = API_BASE.length > 0;

export interface VerifyResponse {
  sessionToken: string;
  email: string;
}

export interface ListeningResponse {
  serverTotalMs: number;
  syncedThroughMs: number;
}

class HttpError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

async function call<T>(
  path: string,
  init: RequestInit & { sessionToken?: string } = {},
): Promise<T> {
  const { sessionToken, headers, ...rest } = init;
  const res = await fetch(`${API_BASE}${path}`, {
    ...rest,
    headers: {
      "Content-Type": "application/json",
      ...(sessionToken ? { Authorization: `Bearer ${sessionToken}` } : {}),
      ...headers,
    },
  });
  if (!res.ok) {
    let message = `request failed (${res.status})`;
    try {
      const body = (await res.json()) as { error?: string };
      if (body.error) message = body.error;
    } catch {
      // non-JSON error body — keep the default message
    }
    throw new HttpError(res.status, message);
  }
  return (await res.json()) as T;
}

export function requestLink(email: string): Promise<{ ok: boolean }> {
  return call("/auth/request", {
    method: "POST",
    body: JSON.stringify({ email }),
  });
}

export function verify(token: string): Promise<VerifyResponse> {
  return call("/auth/verify", {
    method: "POST",
    body: JSON.stringify({ token }),
  });
}

export function logout(sessionToken: string): Promise<{ ok: boolean }> {
  return call("/auth/logout", { method: "POST", sessionToken });
}

export function putListening(
  sessionToken: string,
  deviceId: string,
  deviceTotalMs: number,
  keepalive = false,
): Promise<ListeningResponse> {
  return call("/listening", {
    method: "PUT",
    sessionToken,
    keepalive,
    body: JSON.stringify({ deviceId, deviceTotalMs }),
  });
}

export function getListening(
  sessionToken: string,
): Promise<ListeningResponse> {
  return call("/listening", { method: "GET", sessionToken });
}

export function deleteListening(
  sessionToken: string,
): Promise<{ ok: boolean }> {
  return call("/listening", { method: "DELETE", sessionToken });
}

export function deleteAccount(sessionToken: string): Promise<{ ok: boolean }> {
  return call("/account", { method: "DELETE", sessionToken });
}

export { HttpError };
