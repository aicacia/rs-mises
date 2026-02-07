import { env } from '$env/dynamic/public';
import { createChannel, FetchTransport } from 'nice-grpc-web';
import { isTauri } from './isTauri';
import { once } from './once';
import { createTauriTransport } from './createTauriTransport';

function createTransport() {
	if (isTauri()) {
		return createTauriTransport();
	}
	return FetchTransport();
}

export const channel = once(() => createChannel(env.PUBLIC_GRPC_MISES_API_URL, createTransport()));
