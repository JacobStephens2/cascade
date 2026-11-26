import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

// The standalone /auth page (public/auth/index.html) ships without the
// player bundle, so its inline script hand-mirrors src/sync/signInLink.ts.
// Run that script here against a mocked DOM so the two parsers cannot
// drift apart silently: this page, not the shell, is what emailed links
// land on (see navigateFallbackDenylist).

const html = readFileSync(
  new URL("./public/auth/index.html", import.meta.url),
  "utf8",
);
const script = html.match(/<script>([\s\S]*?)<\/script>/)?.[1];
assert.ok(script, "standalone /auth page has an inline script");

interface FakeElement {
  textContent: string;
  innerHTML: string;
  hidden: boolean;
  href: string;
  addEventListener: () => void;
}

function fakeElement(): FakeElement {
  return {
    textContent: "",
    innerHTML: "",
    hidden: false,
    href: "",
    addEventListener: () => {},
  };
}

interface PageRun {
  token: FakeElement;
  lead: FakeElement;
  open: FakeElement;
  content: FakeElement;
  location: { hash: string; search: string; pathname: string; href: string };
  replacedWith: string[];
}

function runPage(locationInit: { hash?: string; search?: string }): PageRun {
  const elements: Record<string, FakeElement> = {
    token: fakeElement(),
    copy: fakeElement(),
    lead: fakeElement(),
    open: fakeElement(),
    content: fakeElement(),
  };
  elements.open.hidden = true;
  const location = {
    hash: "",
    search: "",
    pathname: "/auth/",
    href: "https://cascade.stephens.page/auth/",
    ...locationInit,
  };
  const replacedWith: string[] = [];
  vm.runInContext(script!, vm.createContext({
    URLSearchParams,
    location,
    history: {
      replaceState: (_state: unknown, _title: string, url: string) =>
        replacedWith.push(url),
    },
    document: { getElementById: (id: string) => elements[id] },
    navigator: { clipboard: { writeText: async () => {} } },
  }));
  return { ...elements, location, replacedWith } as PageRun;
}

test("a fragment token is shown and the fragment leaves the address bar", () => {
  const page = runPage({ hash: "#token=abc123" });
  assert.equal(page.token.textContent, "abc123");
  assert.deepEqual(page.replacedWith, ["/auth/"]);
  assert.equal(page.open.hidden, true);
});

test("a windows fragment link offers the cascade:// handoff", () => {
  const page = runPage({ hash: "#token=abc123&app=windows" });
  assert.equal(page.token.textContent, "abc123");
  assert.equal(page.open.hidden, false);
  assert.equal(page.open.href, "cascade://auth?token=abc123");
  assert.equal(page.location.href, "cascade://auth?token=abc123");
  assert.deepEqual(page.replacedWith, ["/auth/"]);
});

test("an older query-string token is still shown", () => {
  const page = runPage({ search: "?token=abc123" });
  assert.equal(page.token.textContent, "abc123");
  assert.deepEqual(page.replacedWith, ["/auth/"]);
});

test("a link with no token anywhere says so", () => {
  const page = runPage({});
  assert.match(page.content.innerHTML, /missing its token/);
  assert.deepEqual(page.replacedWith, []);
});
