export function callbackUrlFromRequestUrl<T = unknown>(requestUrl: URL, response?: T) {
	const callbackUrlParam = requestUrl.searchParams.get('callback_url');
	if (!callbackUrlParam) {
		throw new Error('Missing `callback_url` parameter');
	}
	const stateParam = requestUrl.searchParams.get('state');
	if (!stateParam) {
		throw new Error('Missing `state` parameter');
	}
	const callbackUrl = new URL(callbackUrlParam);
	callbackUrl.searchParams.set('state', stateParam);
	if (response) {
		callbackUrl.searchParams.set('response', JSON.stringify(response));
	}
	return callbackUrl;
}
