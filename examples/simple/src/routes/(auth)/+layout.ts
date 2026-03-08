import type { LayoutLoad } from './$types';
import { getOidcClient } from '$lib/common/state/user.svelte';

export const load: LayoutLoad = async (event) => {
	await event.parent();
	const oidcClient = getOidcClient();
	// const user = await oidcClient.getUser();

	return {
		user: {
			profile: {
				preferred_username: 'User'
			}
		}
	};
};
