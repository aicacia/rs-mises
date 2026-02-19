import type { TokenResponse, UserInfo } from '$lib/proto/mises';
import { createStorage } from '@aicacia/svelte-headless';
import { oidcClient, setAuthorizationToken } from '$lib/common/util/grpcClient';

// persistent storage wrappers (use createStorage so UI code is testable and reactive)
const tokenResponseStorage = createStorage<TokenResponse | null>('mises_token_response', null);

export function getTokenResponse(): TokenResponse | null {
	return tokenResponseStorage.item;
}

export function setTokenResponse(tokenResponse: TokenResponse | null): void {
	tokenResponseStorage.item = tokenResponse;
	setAuthorizationToken(tokenResponse?.accessToken ?? null);
}

export async function getCurrentUserInfo(): Promise<UserInfo | null> {
	const tokenResponse = tokenResponseStorage.item;
	if (!tokenResponse) {
		return null;
	}

	try {
		return await oidcClient().getUserInfo({});
	} catch (e) {
		console.error('getCurrentUserInfo error', e);
		return null;
	}
}

export async function logout(): Promise<void> {
	setTokenResponse(null);
}
