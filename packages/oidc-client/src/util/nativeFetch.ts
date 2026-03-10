import { generateState } from './generateState.js';
import { openUrl } from './openUrl.js';

export type NativeFetchOptions = {
	callbackUrl?: string;
	timeout?: number;
};

/**
 * Opens a native protocol URL and waits for the native app to respond
 * by opening a callback URL with the response data.
 *
 * @param url - The native protocol URL to open (e.g., mises://register-client)
 * @param options - Configuration options
 * @returns Promise that resolves when the native app responds via callback
 */
export async function nativeFetch<T = unknown>(
	url: string | URL,
	options: NativeFetchOptions = {}
): Promise<T> {
	if (typeof window === 'undefined') {
		throw new Error('nativeFetch can only be used in browser environments');
	}

	const originUrl = window.location.origin;
	const urlObj = typeof url === 'string' ? new URL(url) : url;
	const state = generateState();
	const callbackUrl = options.callbackUrl ?? `${originUrl}/native-callback`;
	const timeout = options.timeout;

	urlObj.searchParams.set('native_state', state);
	urlObj.searchParams.set('callback_url', callbackUrl);

	return new Promise<T>((resolve, reject) => {
		let popupWindow: Window | null = null;
		let timeoutId: ReturnType<typeof setTimeout> | null = null;
		let messageListener: ((event: MessageEvent) => void) | null = null;
		let storageListener: ((event: StorageEvent) => void) | null = null;

		const cleanup = () => {
			if (timeoutId) clearTimeout(timeoutId);
			if (messageListener) {
				window.removeEventListener('message', messageListener);
			}
			if (storageListener) {
				window.removeEventListener('storage', storageListener);
			}
			if (popupWindow && !popupWindow.closed) {
				popupWindow.close();
			}
		};

		if (timeout) {
			timeoutId = setTimeout(() => {
				cleanup();
				reject(new Error(`Native fetch timeout after ${timeout}ms`));
			}, timeout);
		}

		messageListener = (event: MessageEvent) => {
			if (event.origin !== originUrl) {
				return;
			}
			if (event.data?.type !== 'native-fetch-response') {
				return;
			}
			if (event.data?.state !== state) {
				return;
			}
			cleanup();
			resolve(event.data.data as T);
		};

		storageListener = (event: StorageEvent) => {
			const storageKey = `native-fetch-response-${state}`;
			if (event.key !== storageKey || event.newValue == null) {
				return;
			}

			localStorage.removeItem(storageKey);
			cleanup();
			try {
				const data = JSON.parse(event.newValue);
				resolve(data as T);
			} catch (error) {
				reject(new Error(`Failed to parse native fetch response: ${error}`));
			}
		};

		window.addEventListener('message', messageListener);
		window.addEventListener('storage', storageListener);

		popupWindow = openUrl(urlObj, {
			popup: true
		});
	});
}

/**
 * Helper to handle native fetch callback in the callback page.
 * Call this in your callback route to send the response back to the waiting fetch.
 *
 * @param searchParams - URLSearchParams from the callback URL
 */
export function handleNativeFetchCallback(searchParams: URLSearchParams): void {
	const state = searchParams.get('native_state');
	if (!state) {
		console.warn('No state parameter in native fetch callback');
		return;
	}
	const response = searchParams.get('response') ?? 'null';

	if (window.opener && !window.opener.closed) {
		window.opener.postMessage(
			{
				type: 'native-fetch-response',
				state,
				data: JSON.parse(response)
			},
			'*'
		);
		window.close();
	} else {
		localStorage.setItem(`native-fetch-response-${state}`, response);
		if (window.history.length > 1) {
			window.history.back();
		}
	}
}
