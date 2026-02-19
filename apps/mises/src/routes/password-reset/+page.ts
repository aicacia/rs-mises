import type { PageLoad } from './$types';
import { getCurrentUserInfo, requiresPasswordReset } from '$lib/common/state/auth.svelte';
import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';

export const load: PageLoad = async () => {
	const userInfo = await getCurrentUserInfo();

	if (!userInfo) {
		redirect(302, resolve('/signin'));
	}

	if (!requiresPasswordReset()) {
		redirect(302, resolve('/'));
	}

	return { userInfo };
};
