import { oidcClient } from "$lib/common/util/grpcClient";
import type { AuthorizeRequest, Client, ClientRegisterRequest } from "$lib/proto/mises";

export type ClientInfo = ClientRegisterRequest;

function deepEqual(a: unknown, b: unknown): boolean {
	if (a === b) {
		return true;
	}
	if (Array.isArray(a) && Array.isArray(b)) {
		if (a.length !== b.length) {
			return false;
		}
		for (let i = 0; i < a.length; i++) {
			if (!deepEqual(a[i], b[i])) {
				return false;
			}
		}
		return true;
	}
	if (a !== null && b !== null && typeof a === 'object' && typeof b === 'object') {
		const aObj = a as Record<string, unknown>;
		const bObj = b as Record<string, unknown>;
		const aKeys = Object.keys(aObj);
		const bKeys = Object.keys(bObj);
		if (aKeys.length !== bKeys.length) {
			return false;
		}
		for (const key of aKeys) {
			if (!bKeys.includes(key)) {
				return false;
			}
			if (!deepEqual(aObj[key], bObj[key])) {
				return false;
			}
		}
		return true;
	}
	return false;
}

export function getClientDiff(client: Client, clientInfo: ClientInfo): Partial<ClientInfo> | false {
	const diff: Partial<ClientInfo> = {};
	let changed = false;

	for (const key of Object.keys(clientInfo) as (keyof ClientInfo)[]) {
		const a = client[key];
		const b = clientInfo[key];

		if (!deepEqual(a, b)) {
			changed = true;
			diff[key] = b as never;
		}
	}

	if (!changed) {
		return false;
	}

	return diff;
}

export function rejectAuthorizeRequest(
	authorizeRequest: Pick<AuthorizeRequest, 'redirectUri' | 'state' | 'nonce'>,
	error: string,
	errorDescription: string
) {
	const url = new URL(authorizeRequest.redirectUri!);
	if (authorizeRequest.state) {
		url.searchParams.append('state', authorizeRequest.state);
	}
	if (authorizeRequest.nonce) {
		url.searchParams.append('nonce', authorizeRequest.nonce);
	}
	url.searchParams.append('error', error);
	url.searchParams.append('error_description', errorDescription);
	window.location.href = url.toString();
}

export async function resolveAuthorizeRequest(authorizeRequest: AuthorizeRequest) {
	const url = new URL(authorizeRequest.redirectUri!);
	if (authorizeRequest.state) {
		url.searchParams.append('state', authorizeRequest.state);
	}
	if (authorizeRequest.nonce) {
		url.searchParams.append('nonce', authorizeRequest.nonce);
	}
	const authorizeResponse = await oidcClient().authorize(authorizeRequest);
	switch (authorizeRequest.responseMode) {
		case 'fragment':
		case 'query': {
			switch (authorizeResponse.type) {
				case 'authorization_code': {
					url.searchParams.set('code', authorizeResponse.code);
					break;
				}
				case 'implicit':
				case 'hybrid': {
					url.searchParams.set('access_token', authorizeResponse.accessToken);
					url.searchParams.set('token_type', authorizeResponse.tokenType);
					url.searchParams.set('expires_in', authorizeResponse.expiresIn);
					if (authorizeResponse.idToken) {
						url.searchParams.set('id_token', authorizeResponse.idToken);
					}
					break;
				}
			}
			window.location.href = url.toString();
			break;
		}
		case 'form_post': {
			throw new Error('not supported yet!');
			break;
		}
		case 'web_message': {
			throw new Error('not supported yet!');
			break;
		}
	}
}
