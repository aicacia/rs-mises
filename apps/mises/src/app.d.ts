// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	interface Window {
		// NOTE: not a complete type definition just used for checking if isTauri
		__TAURI_INTERNALS__?: {
			ipc: (message: {
				cmd: string;
				callback: number;
				error: number;
				payload: unknown;
				options?: InvokeOptions;
			}) => void;
		};
	}
}

export {};
