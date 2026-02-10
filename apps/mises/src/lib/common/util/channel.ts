import { env } from '$env/dynamic/public';
import { createChannel, FetchTransport } from 'nice-grpc-web';
import { once } from './once';
import { createTauriTransport } from './createTauriTransport';
import { isTauri } from '@tauri-apps/api/core';

function createTransport() {
	if (isTauri()) {
		return createTauriTransport();
	}
	return FetchTransport();
}

export const channel = once(() => createChannel(env.PUBLIC_GRPC_MISES_API_URL, createTransport()));
