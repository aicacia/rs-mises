import { handleNativeFetchCallback } from '@aicacia/oidc-client';
import type { PageLoad } from './$types';

// IMPORTANT: this must be run in the browser context, so we disable SSR and prerendering
export const prerender = false;
export const ssr = false;

export const load: PageLoad = async ({ url }) => {
	handleNativeFetchCallback(url.searchParams);
	return {};
};
