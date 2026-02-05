import { env } from '$env/dynamic/public';
import { createChannel, FetchTransport } from 'nice-grpc-web';
import { isTauri } from './isTauri';
import { once } from './once';
import { createTransportTransport } from './createTransportTransport';

function createTransport() {
	if (isTauri()) {
		return createTransportTransport();
	}
	return FetchTransport();
}

export const channel = once(() => createChannel(env.PUBLIC_GRPC_MISES_API_URL, createTransport()));
