/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/client" />

interface ImportMetaEnv {
  /** Base URL of the cascade-sync-server. Empty/undefined disables sync. */
  readonly VITE_SYNC_API?: string;
}

declare module "*.wasm?url" {
  const src: string;
  export default src;
}
