import type { Effect, ServerRequest, ServerResponse } from "../core/types";
import type { DispatchOptions } from "../core/useCascade";
import { SYNC_API_BASE, syncAvailable } from "./config";

/** Dispatches a settle; that dispatch carries the settle's own effects. */
type Settle = (response: ServerResponse) => void;

/** How long a request may take before it is given up and settled as status 0. */
const REQUEST_TIMEOUT_MS = 30_000;

/**
 * Send one request exactly as the core described it and answer with the
 * `serverResponse` that settles it: the HTTP status and body, or status 0
 * when there is no sync server, the network failed or it timed out.
 */
async function send(
  request: ServerRequest,
  keepalive: boolean,
): Promise<ServerResponse> {
  const response = (status: number, body = ""): ServerResponse => ({
    type: "serverResponse",
    id: request.id,
    status,
    body,
  });
  if (!syncAvailable) return response(0);
  try {
    const res = await fetch(`${SYNC_API_BASE}${request.path}`, {
      method: request.method,
      headers: {
        ...(request.body !== undefined
          ? { "Content-Type": "application/json" }
          : {}),
        ...(request.bearerToken
          ? { Authorization: `Bearer ${request.bearerToken}` }
          : {}),
      },
      body: request.body,
      keepalive,
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
    return response(res.status, await res.text().catch(() => ""));
  } catch {
    return response(0);
  }
}

/**
 * The one request carrier. Every dispatch hands it its effects, whichever
 * command produced them (a tick's push, a sign-in's refresh, an account call);
 * it sends each `serverRequest` over HTTP and settles it through `settle`,
 * which is itself a dispatch, so a settle's answer comes back here too. Every
 * request is settled, or the core won't start another.
 */
export function carryRequests(
  effects: Effect[],
  settle: Settle,
  { keepalive = false }: DispatchOptions = {},
): void {
  for (const effect of effects) {
    if (effect.type === "serverRequest")
      void send(effect, keepalive).then(settle);
  }
}
