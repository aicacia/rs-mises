import { Metadata, type MetadataValue } from 'nice-grpc-web';
import type { Frame, Transport } from 'nice-grpc-web/lib/client/Transport';
import type { Event } from '@tauri-apps/api/event';
import { err, ok, type Result } from '@aicacia/trycatch';

type TransportMetadata = { [K in string]: MetadataValue<K>[] };

type TransportFrame =
	| { type: 'header'; header: TransportMetadata }
	| { type: 'data'; data: number[] }
	| { type: 'trailer'; trailer: TransportMetadata };
type TransportEventPayload = { request_id: string } & (
	| { data: TransportFrame }
	| { error: unknown }
);

function transportFrameToFrame(frame: TransportFrame): Frame {
	switch (frame.type) {
		case 'header':
			return { type: 'header', header: new Metadata(frame.header) };
		case 'data':
			return { type: 'data', data: new Uint8Array(frame.data) };
		case 'trailer':
			return { type: 'trailer', trailer: new Metadata(frame.trailer) };
	}
}

interface TransportTransportOptions {
	responseTimeoutMs?: number;
}

export function createTransportTransport({
	responseTimeoutMs = 60000
}: TransportTransportOptions = {}): Transport {
	const event = import('@tauri-apps/api/event');
	const core = import('@tauri-apps/api/core');

	return async function* transportTransport({ body, metadata, method }) {
		const { invoke } = await core;
		const { listen } = await event;

		const chunks: Uint8Array[] = [];
		let totalLength = 0;
		for await (const chunk of body) {
			chunks.push(chunk);
			totalLength += chunk.length;
		}
		const requestBody = new Uint8Array(totalLength);
		let offset = 0;
		for (const chunk of chunks) {
			requestBody.set(chunk, offset);
			offset += chunk.length;
		}

		const queue: Array<Result<Frame, Error>> = [];
		let wake: (() => void) | null = null;

		const enqueue = (item: Result<Frame, Error>) => {
			queue.push(item);
			wake?.();
		};

		const unlistenPromise = listen('grpc-response', async (event: Event<TransportEventPayload>) => {
			try {
				const payload = event.payload;
				if (!payload || payload.request_id === undefined) {
					return;
				}
				if (payload.request_id !== (await requestId)) {
					return;
				}

				if ('data' in payload) {
					const frame = transportFrameToFrame(payload.data);
					enqueue(ok(frame));
				} else if ('error' in payload) {
					enqueue(
						err(
							new Error(
								typeof payload.error === 'string' ? payload.error : JSON.stringify(payload.error)
							)
						)
					);
				}
			} catch (e) {
				enqueue(
					err(
						typeof e === 'string'
							? new Error(e)
							: e instanceof Error
								? e
								: new Error('Unknown error')
					)
				);
			}
		});

		const requestId = invoke<string>('grpc', {
			body: requestBody,
			path: method.path,
			metadata
		});

		try {
			while (true) {
				if (queue.length === 0) {
					await new Promise<void>((resolve, reject) => {
						const id = setTimeout(() => {
							if (wake) {
								wake = null;
							}
							reject(new Error('gRPC response timed out waiting for frames'));
						}, responseTimeoutMs);

						wake = () => {
							clearTimeout(id);
							resolve();
							wake = null;
						};
					});
				}

				const [frame, err] = queue.shift()!;
				if (err) {
					throw err;
				}

				yield frame;

				if (frame.type === 'trailer') {
					break;
				}
			}
		} finally {
			const unlisten = await unlistenPromise;
			unlisten();
		}
	};
}
