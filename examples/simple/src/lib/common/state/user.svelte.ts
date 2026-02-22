import { UserManager, type UserManagerSettings } from 'oidc-client-ts';
import { browser } from '$app/environment';
import icon256x256Png from '$lib/assets/icon256x256.png';
import { env } from '$env/dynamic/public';

const userSettings = async () =>
	browser
		? ({
				authority: env.PUBLIC_MISES_URL,
				client_id: '',
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
				filterProtocolClaims: true,
				extraQueryParams: {
					service_id: 'mises-simple-example',
					registration: JSON.stringify({
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
						terms_of_service_uri: `${env.PUBLIC_URL}${'/terms'}`,
						application_type: 'web',
						auth_method: 'none',
						grant_types: ['authorization_code', 'refresh_token'],
						response_types: ['code'],
						scopes: ['openid', 'profile', 'address', 'offline', 'email', 'phone'],
						audience: [`${env.PUBLIC_URL}`],
						access_token_expires_in_seconds: 3600,
						id_token_expires_in_seconds: 3600,
						refresh_expires_in_seconds: 604800
					})
				}
			} satisfies UserManagerSettings)
		: ({} as never);

const userManager = $derived.by(async () => new UserManager({ ...(await userSettings()) }));

export async function getUserManager() {
	return await userManager;
}
