/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_WS_BASE_URL?: string;
	readonly VITE_API_BASE_URL?: string;
	readonly VITE_WS_MAX_RETRIES?: string;
	readonly VITE_WS_INITIAL_RETRY_DELAY?: string;
	readonly VITE_WS_MAX_RETRY_DELAY?: string;
}

interface ImportMeta {
	readonly env: ImportMetaEnv;
}
