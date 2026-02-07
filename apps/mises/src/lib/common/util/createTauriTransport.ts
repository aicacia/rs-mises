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

interface TauriTransportOptions {
	responseTimeoutMs?: number;
}

export function createTauriTransport({
	responseTimeoutMs = 60000
}: TauriTransportOptions = {}): Transport {
	const event = import('@tauri-apps/api/event');
	const core = import('@tauri-apps/api/core');

	return async function* transportTransport({ body, metadata, method }) {
		const { invoke } = await core;
		const { listen } = await event;

		console.debug('[tauri-transport] start', { path: method.path, metadata });

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

		console.debug('[tauri-transport] request body built', {
			length: requestBody.length,
			chunks: chunks.length
		});

		const queue: Array<Result<Frame, Error>> = [];
		let wake: (() => void) | null = null;

		const enqueue = (item: Result<Frame, Error>) => {
			queue.push(item);
			wake?.();
		};

		const unlistenPromise = listen('grpc-response', async (event: Event<TransportEventPayload>) => {
			try {
				const payload = event.payload;
				console.debug('[tauri-transport] grpc-response event', payload);
				if (!payload || payload.request_id === undefined) {
					console.debug('[tauri-transport] missing payload or request_id');
					return;
				}
				const expectedId = await requestId;
				if (payload.request_id !== expectedId) {
					console.debug('[tauri-transport] request id mismatch', {
						expectedId,
						request_id: payload.request_id
					});
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
		requestId
			.then((id) => console.debug('[tauri-transport] invoke returned request id', id))
			.catch((e) => console.debug('[tauri-transport] invoke error', e));

		try {
			while (true) {
				if (queue.length === 0) {
					console.debug('[tauri-transport] queue empty, waiting for frames');
					await new Promise<void>((resolve) => {
						const id = setTimeout(() => {
							if (wake) {
								wake = null;
							}
							requestId
								.then((rid) =>
									console.debug('[tauri-transport] timeout waiting for frames', { requestId: rid })
								)
								.catch(() => {});
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
					try {
						const status = frame.trailer.get && frame.trailer.get('grpc-status');
						const msg = frame.trailer.get && frame.trailer.get('grpc-message');
						console.debug('[tauri-transport] trailer received, ending stream', status, msg);
					} catch (e) {
						console.debug(
							'[tauri-transport] trailer received, ending stream (failed to read trailer)',
							e
						);
					}
					break;
				}
			}
		} finally {
			const unlisten = await unlistenPromise;
			unlisten();
		}
	};
}
