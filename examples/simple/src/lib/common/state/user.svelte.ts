import icon256x256Png from '$lib/assets/icon256x256.png';
import { env } from '$env/dynamic/public';
import { createStorage } from '@aicacia/svelte-headless';
import { OidcClient } from '@aicacia/oidc-client';

const clientIdStorage = createStorage<string | null>('mises-simple-example-client-id', null);

const oidcClient = new OidcClient({
	clientConfig: {
		authority: 'mises://app',
		redirectUri: `${env.PUBLIC_URL}/callback`,
		clientId: clientIdStorage.item ?? undefined,
		responseType: 'code',
		registration: {
			clientName: 'Simple Example',
			serviceId: 'mises-simple-example',
			scope: 'openid profile address offline email phone',
			redirectUris: [`${env.PUBLIC_URL}/callback`],
			postLogoutRedirectUris: [`${env.PUBLIC_URL}/logout`],
			logoUri: `${env.PUBLIC_URL}${icon256x256Png}`,
			clientUri: `${env.PUBLIC_URL}`,
			policyUri: `${env.PUBLIC_URL}/policy`,
			tosUri: `${env.PUBLIC_URL}/terms`,
			applicationType: 'web',
			tokenEndpointAuthMethod: 'none',
			grantTypes: ['authorization_code', 'refresh_token'],
			responseTypes: ['code'],
			accessTokenExpiry: 3600,
			refreshTokenExpiry: 604800
		}
	}
});

export function getOidcClient() {
	return oidcClient;
}

export async function signin() {
	const client = oidcClient;
	return await client.signin();
}

oidcClient.on('registered', (registration) => {
	clientIdStorage.item = registration.client_id ?? null;
});
