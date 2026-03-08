import { getOidcClient } from '$lib/common/state/user.svelte';
import type { PageLoad } from './$types';

// IMPORTANT: this must be run in the browser context, so we disable SSR and prerendering
export const prerender = false;
export const ssr = false;

export const load: PageLoad = async ({ url }) => {
	const oidcClient = await getOidcClient();
	oidcClient.handleRegistrationCallback(url.searchParams);
	return {};
};
