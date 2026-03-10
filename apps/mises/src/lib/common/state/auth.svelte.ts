import type { TokenResponse, UserInfo } from '$lib/proto/mises';
import { createStorage } from '@aicacia/svelte-headless';
import { oidcClient, setAuthorizationToken } from '$lib/common/util/grpcClient';
import { browser } from '$app/environment';

const tokenResponseStorage = createStorage<TokenResponse | null>('mises-token', null);

let tokenExpiryTimeout: ReturnType<typeof setTimeout> | null = null;
let tokenGeneration = 0;

export function getTokenResponse(): TokenResponse | null {
	return tokenResponseStorage.item;
}

export function setTokenResponse(tokenResponse: TokenResponse | null): void {
	tokenGeneration += 1;
	const scheduledGeneration = tokenGeneration;

	if (!tokenResponse) {
		clearTokenExpiryTimeout();
		clearAuthState();
		return;
	}

	clearTokenExpiryTimeout();
	const expiresIn = tokenResponse.expiresIn;
	if (expiresIn !== undefined) {
		const expiresInMs = Math.max(0, expiresIn) * 1000;
		tokenExpiryTimeout = setTimeout(() => {
			void onTokenExpired(scheduledGeneration, tokenResponse);
		}, expiresInMs);
	}

	tokenResponseStorage.item = tokenResponse;
}

export async function getCurrentUserInfo(): Promise<UserInfo | null> {
	const tokenResponse = getTokenResponse();
	if (!tokenResponse) {
		console.log('No token response found');
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
			const tokenResponse = getTokenResponse();
			setAuthorizationToken(tokenResponse?.accessToken ?? null);

			return () => {
				clearTokenExpiryTimeout();
			};
		});
	});
}

function clearTokenExpiryTimeout(): void {
	if (tokenExpiryTimeout !== null) {
		clearTimeout(tokenExpiryTimeout);
		tokenExpiryTimeout = null;
	}
}

function clearAuthState(): void {
	tokenResponseStorage.item = null;
}

async function refreshTokenResponse(tokenResponse: TokenResponse): Promise<TokenResponse | null> {
	if (!tokenResponse.refreshToken) {
		return null;
	}

	try {
		const refreshed = await oidcClient().token({
			refreshToken: {
				refreshToken: tokenResponse.refreshToken,
				scope: tokenResponse.scope
			}
		});

		return {
			...refreshed,
			refreshToken: refreshed.refreshToken ?? tokenResponse.refreshToken,
			scope: refreshed.scope ?? tokenResponse.scope
		};
	} catch (e) {
		console.error('refresh token error', e);
		return null;
	}
}

async function onTokenExpired(
	scheduledGeneration: number,
	tokenResponse: TokenResponse
): Promise<void> {
	if (tokenGeneration !== scheduledGeneration) {
		return;
	}

	const refreshed = await refreshTokenResponse(tokenResponse);
	if (tokenGeneration !== scheduledGeneration) {
		return;
	}

	if (refreshed) {
		setTokenResponse(refreshed);
		return;
	}

	setTokenResponse(null);
}
