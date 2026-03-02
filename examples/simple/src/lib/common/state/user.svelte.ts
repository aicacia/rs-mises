import { UserManager, type UserManagerSettings } from 'oidc-client-ts';
import { browser } from '$app/environment';
import icon256x256Png from '$lib/assets/icon256x256.png';
import { env } from '$env/dynamic/public';
import { createStorage } from '@aicacia/svelte-headless';

const clientId = createStorage<string | null>('mises-simple-example-client-id', null);

// if we've been redirected back from a registration flow, persist the new client id
if (browser) {
	const params = new URL(window.location.href).searchParams;
	const returned = params.get('client_id');
	if (returned) {
		clientId.set(returned);
	}

	// if we still don't have a client id, kick off the registration flow
	if (!clientId.item) {
		const registrationInfo = {
			name: 'Simple Example',
			service_id: 'mises-simple-example',
			redirect_uris: [
				`${env.PUBLIC_URL}/callback`,
				`${env.PUBLIC_URL}/popup-callback`,
				`${env.PUBLIC_URL}/silent-callback`
			],
			post_logout_redirect_uris: [`${env.PUBLIC_URL}/logout`],
			logo_uri: `${window.location.origin}${icon256x256Png}`,
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

		const u = new URL(`${env.PUBLIC_MISES_URL}/register`);
		u.searchParams.set('registration', JSON.stringify(registrationInfo));
		u.searchParams.set('redirect_uri', `${env.PUBLIC_URL}`);
		window.location.href = u.toString();
	}
}

const userSettings = async () =>
	browser
		? ({
				authority: env.PUBLIC_MISES_URL,
				client_id: clientId.item ?? 'unknown',
				redirect_uri: `${env.PUBLIC_URL}/callback`,
				post_logout_redirect_uri: `${env.PUBLIC_URL}/logout`,
				response_type: 'code',
				scope: 'openid profile offline',
				response_mode: 'query',
				loadUserInfo: true,
				popup_redirect_uri: `${env.PUBLIC_URL}/popup-callback`,
				popup_post_logout_redirect_uri: `${env.PUBLIC_URL}/popup-callback`,
				silent_redirect_uri: `${env.PUBLIC_URL}/silent-callback`,
				automaticSilentRenew: true,
				filterProtocolClaims: true
			} satisfies UserManagerSettings)
		: ({} as never);

const userManager = $derived.by(async () =>
	patchJsonService(new UserManager({ ...(await userSettings()) }))
);

export async function getUserManager() {
	return await userManager;
}

function patchJsonService(userManager: UserManager): UserManager {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function buildRedirectUrl(url: string, body: any): string {
		const params = new URLSearchParams(body).toString();
		return `${url}?${params}`;
	}

	console.log(userManager);
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	(userManager as any)._client._validator._tokenClient._jsonService.postForm = (
		url: string,
		body: unknown,
		_timeout: number,
		_credentials?: RequestCredentials
	): Promise<unknown> => {
		window.location.href = buildRedirectUrl(url, body);
		return new Promise(() => {});
	};

	return userManager;
}
