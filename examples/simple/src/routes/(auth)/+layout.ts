import { resolve } from '$app/paths';
import { redirect } from '@sveltejs/kit';
import { OidcClientError } from '@aicacia/oidc-client';
import type { LayoutLoad } from './$types';
import { getOidcClient } from '$lib/common/state/user.svelte';

export const load: LayoutLoad = async (event) => {
	await event.parent();
	const oidcClient = getOidcClient();
	try {
		const user = await oidcClient.getUserInfo();

		console.log(user);

		return {
			user
		};
	} catch (e) {
		if (e instanceof OidcClientError) {
			if (e.code === 'NO_ACCESS_TOKEN') {
				redirect(302, resolve('/signin'));
			}
		}
		throw e;
	}
};
