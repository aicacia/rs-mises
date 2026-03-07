import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';
import { resolve } from '$app/paths';
import { needsRegistration } from '$lib/common/state/user.svelte';

export const load: LayoutLoad = async (event) => {
	await event.parent();
	if (needsRegistration()) {
		redirect(302, resolve('/signin'));
	}

	return {
		user: {
			profile: {
				preferred_username: 'User'
			}
		}
	};
};
