import assert from "node:assert/strict";
import test from "node:test";

// Workbox's router expects a service-worker global, and its dev build logs
// unless this is production. Set both before importing it.
process.env.NODE_ENV = "production";
(globalThis as { self?: typeof globalThis }).self = globalThis;

const { NavigationRoute } = await import("workbox-routing");
const { navigateFallbackDenylist } = await import("./vite.config.ts");

/**
 * Whether the service worker's navigation fallback would answer this URL
 * with the player shell. Workbox tests the denylist against pathname + search.
 */
function playerShellHandles(pathWithQueryAndFragment: string): boolean {
  const route = new NavigationRoute(() => new Response("shell"), {
    denylist: navigateFallbackDenylist,
  });
  const url = new URL(pathWithQueryAndFragment, "https://cascade.stephens.page");
  return (
    route.match({
      url,
      request: { mode: "navigate" },
    }) === true
  );
}

test("a sign-in link is not answered with the player shell", () => {
  assert.equal(playerShellHandles("/auth#token=one-time"), false);
  assert.equal(playerShellHandles("/auth/#token=one-time"), false);
  assert.equal(playerShellHandles("/auth#token=one-time&app=windows"), false);
  assert.equal(playerShellHandles("/auth?token=one-time"), false);
  assert.equal(playerShellHandles("/auth/?token=one-time"), false);
  assert.equal(playerShellHandles("/auth"), false);
  assert.equal(playerShellHandles("/auth/"), false);
});

test("the player itself still uses the navigation fallback", () => {
  assert.equal(playerShellHandles("/"), true);
});
