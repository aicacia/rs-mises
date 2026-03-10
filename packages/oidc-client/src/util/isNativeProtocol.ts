export function isNativeProtocol(url: URL): boolean {
	const protocol = url.protocol.toLowerCase();
	return protocol !== 'http:' && protocol !== 'https:' && protocol !== 'about:';
}