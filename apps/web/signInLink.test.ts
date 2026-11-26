import assert from "node:assert/strict";
import test from "node:test";

const { parseSignInLink } = await import("./src/sync/signInLink.ts");

test("the token is read from the fragment and the fragment is cleaned away", () => {
  assert.deepEqual(
    parseSignInLink("https://cascade.stephens.page/auth#token=abc123"),
    {
      token: "abc123",
      windowsHandoff: false,
      cleanedHref: "https://cascade.stephens.page/auth",
    },
  );
});

test("a windows link keeps its app hint, from the fragment", () => {
  assert.deepEqual(
    parseSignInLink("https://cascade.stephens.page/auth#token=abc123&app=windows"),
    {
      token: "abc123",
      windowsHandoff: true,
      cleanedHref: "https://cascade.stephens.page/auth",
    },
  );
  assert.deepEqual(
    parseSignInLink("https://cascade.stephens.page/auth#app=windows&token=abc123"),
    {
      token: "abc123",
      windowsHandoff: true,
      cleanedHref: "https://cascade.stephens.page/auth",
    },
  );
});

test("an older query-string link still parses and is cleaned the same way", () => {
  assert.deepEqual(
    parseSignInLink("https://cascade.stephens.page/auth?token=abc123&app=windows"),
    {
      token: "abc123",
      windowsHandoff: true,
      cleanedHref: "https://cascade.stephens.page/auth",
    },
  );
});

test("a percent-encoded fragment token is decoded", () => {
  assert.equal(
    parseSignInLink("https://cascade.stephens.page/auth#token=a%2Fb%3D")?.token,
    "a/b=",
  );
});

test("no token, no sign-in link", () => {
  assert.equal(parseSignInLink("https://cascade.stephens.page/auth"), null);
  assert.equal(parseSignInLink("https://cascade.stephens.page/auth#token="), null);
  assert.equal(parseSignInLink("https://cascade.stephens.page/"), null);
});
