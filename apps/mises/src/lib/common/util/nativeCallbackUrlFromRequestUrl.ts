export function nativeCallbackUrlFromRequestUrl<T = unknown>(requestUrl: URL, response: T | null) {
	const callbackUrlParam = requestUrl.searchParams.get('callback_url');
	if (!callbackUrlParam) {
		throw new Error('Missing `callback_url` parameter');
	}
	const nativeStateParam = requestUrl.searchParams.get('native_state');
	if (!nativeStateParam) {
		throw new Error('Missing `native_state` parameter');
	}
	const callbackUrl = new URL(callbackUrlParam);
	callbackUrl.searchParams.set('native_state', nativeStateParam);
	callbackUrl.searchParams.set('response', JSON.stringify(response));
	return callbackUrl;
}
