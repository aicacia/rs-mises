import type { TokenResponse, UserInfo } from '$lib/proto/mises';
import { createStorage } from '@aicacia/svelte-headless';
import { oidcClient, setAuthorizationToken } from '$lib/common/util/grpcClient';
import { browser } from '$app/environment';

const tokenResponseStorage = createStorage<TokenResponse | null>('mises-token', null);

export function getTokenResponse(): TokenResponse | null {
	return tokenResponseStorage.item;
}

export function setTokenResponse(tokenResponse: TokenResponse | null): void {
	tokenResponseStorage.item = tokenResponse;
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

if (browser) {
	$effect.root(() => {
		$effect(() => {
			setAuthorizationToken(tokenResponseStorage.item?.accessToken ?? null);
		});
	});
}
