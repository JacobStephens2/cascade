/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

declare module "*.wasm?url" {
  const src: string;
  export default src;
}
