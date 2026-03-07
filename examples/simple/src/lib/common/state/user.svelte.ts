import { browser } from '$app/environment';
import icon256x256Png from '$lib/assets/icon256x256.png';
import { env } from '$env/dynamic/public';
import { createStorage } from '@aicacia/svelte-headless';
import { OidcClient } from '@aicacia/oidc-client';

const clientId = createStorage<string | null>('mises-simple-example-client-id', null);

const oidcClient = $derived.by(
	() =>
		new OidcClient({
			clientConfig: {
				authority: 'mises://app',
				redirectUri: `${env.PUBLIC_URL}/callback`,
				clientId: clientId.item ?? undefined,
				responseType: 'code',
				scope: 'openid profile address offline email phone'
			}
		})
);

if (browser) {
	const returned = oidcClient.handleRegistrationCallback(window.location.href);
	if (returned) {
		clientId.update(() => returned);
	}
}

export async function getOidcClient() {
	return oidcClient;
}

export async function startRegistration() {
	try {
		const registrationInfo = {
			name: 'Simple Example',
			service_id: 'mises-simple-example',
			redirect_uris: [`${env.PUBLIC_URL}/callback`],
			post_logout_redirect_uris: [`${env.PUBLIC_URL}/logout`],
			logo_uri: `${typeof window !== 'undefined' ? window.location.origin : ''}${icon256x256Png}`,
			client_uri: `${env.PUBLIC_URL}`,
			policy_uri: `${env.PUBLIC_URL}/policy`,
			terms_of_service_uri: `${env.PUBLIC_URL}/terms`,
			application_type: 'web',
			auth_method: 'none',
			grant_types: ['authorization_code', 'refresh_token'],
			response_types: ['code'],
			scope: 'openid profile address offline email phone',
			audience: [`${env.PUBLIC_URL}`],
			access_token_expires_in_seconds: 3600,
			id_token_expires_in_seconds: 3600,
			refresh_expires_in_seconds: 604800
		};

		await oidcClient.startRegistration({
			redirectUri: `${env.PUBLIC_URL}`,
			registration: registrationInfo
		});
	} catch (error) {
		console.error('Registration not available:', error);
		throw error;
	}
}

export function setClientId(id: string, secret?: string) {
	clientId.update(() => id);
	oidcClient.setClientId(id, secret);
}

export async function signin(usePopup = false) {
	const client = oidcClient;
	return await client.signin({ popup: usePopup });
}

export function needsRegistration() {
	return !clientId.item;
}

export async function getAuthorizationUrl() {
	const client = oidcClient;
	return await client.getAuthorizationUrl();
}
