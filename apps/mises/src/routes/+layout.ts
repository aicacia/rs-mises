import { configurationClient } from '$lib/common/util/grpcClient';
import type { LayoutLoad } from './$types';

export const prerender = true;
export const ssr = false;

export const load: LayoutLoad = async () => {
	const configuration = await configurationClient().get({});

	return {
		configuration
	};
};
