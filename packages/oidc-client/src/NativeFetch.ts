import { generateState } from './util/generateState.js';
import { openUrl } from './util/openUrl.js';

export type NativeFetchOptions = {
	callbackUrl?: string;
	timeout?: number;
	stateParam?: string;
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

	const urlObj = typeof url === 'string' ? new URL(url) : url;
	const state = generateState();
	const callbackUrl = options.callbackUrl || `${window.location.origin}/native-callback`;
	const timeout = options.timeout || 60000;
	const stateParam = options.stateParam || 'state';

	urlObj.searchParams.set(stateParam, state);
	urlObj.searchParams.set('callback_url', callbackUrl);

	return new Promise<T>((resolve, reject) => {
		let popupWindow: Window | null = null;
		let timeoutId: ReturnType<typeof setTimeout> | null = null;
		let checkInterval: ReturnType<typeof setInterval> | null = null;
		let messageListener: ((event: MessageEvent) => void) | null = null;

		const cleanup = () => {
			if (timeoutId) clearTimeout(timeoutId);
			if (checkInterval) clearInterval(checkInterval);
			if (messageListener) {
				window.removeEventListener('message', messageListener);
			}
			if (popupWindow && !popupWindow.closed) {
				popupWindow.close();
			}
		};

		timeoutId = setTimeout(() => {
			cleanup();
			reject(new Error(`Native fetch timeout after ${timeout}ms`));
		}, timeout);

		messageListener = (event: MessageEvent) => {
			if (event.data?.type === 'native-fetch-response' && event.data?.state === state) {
				cleanup();
				resolve(event.data.data as T);
			}
		};

		window.addEventListener('message', messageListener);

		checkInterval = setInterval(() => {
			const storageKey = `native-fetch-response-${state}`;
			const storedResponse = localStorage.getItem(storageKey);
			if (storedResponse) {
				localStorage.removeItem(storageKey);
				cleanup();
				try {
					const data = JSON.parse(storedResponse);
					resolve(data as T);
				} catch (error) {
					reject(new Error(`Failed to parse native fetch response: ${error}`));
				}
			}
		}, 100);

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
	const state = searchParams.get('state');
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
