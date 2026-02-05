import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { resolve } from '$app/paths';

export const load: PageLoad = async () => {
	redirect(302, resolve('/signin'));
};
