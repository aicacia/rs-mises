import { oidcClient } from '$lib/common/util/grpcClient';
import type { AuthorizeRequest, Client, ClientRegisterRequest } from '$lib/proto/mises';
import { isTauri } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

// helper utilities used by authorization pages

// url helpers

export function rejectAuthorizeRequest(
	authorizeRequest: Pick<AuthorizeRequest, 'redirectUri' | 'state' | 'nonce'>,
	error: string,
	errorDescription: string
) {
	const url = new URL(authorizeRequest.redirectUri!);
	if (authorizeRequest.state) {
		url.searchParams.append('state', authorizeRequest.state);
	}
	if (authorizeRequest.nonce) {
		url.searchParams.append('nonce', authorizeRequest.nonce);
	}
	url.searchParams.append('error', error);
	url.searchParams.append('error_description', errorDescription);
	window.location.href = url.toString();
}

export async function resolveAuthorizeRequest(authorizeRequest: AuthorizeRequest) {
	console.log(authorizeRequest);

	const authorizeResponse = await oidcClient().authorize(authorizeRequest);

	const url = new URL(authorizeResponse.redirectUri!);
	if (authorizeRequest.state) {
		url.searchParams.append('state', authorizeRequest.state);
	}
	if (authorizeRequest.nonce) {
		url.searchParams.append('nonce', authorizeRequest.nonce);
	}

	if (isTauri()) {
		await openUrl(url.toString());
	} else {
		window.location.href = url.toString();
	}
	return;
	// TODO: change response type to include all possible response parameters, and handle them accordingly
	switch (authorizeRequest.responseMode) {
		case 'fragment':
		case 'query': {
			switch (authorizeResponse.type) {
				case 'authorization_code': {
					url.searchParams.set('code', authorizeResponse.code);
					break;
				}
				case 'implicit':
				case 'hybrid': {
					url.searchParams.set('access_token', authorizeResponse.accessToken);
					url.searchParams.set('token_type', authorizeResponse.tokenType);
					url.searchParams.set('expires_in', authorizeResponse.expiresIn);
					if (authorizeResponse.idToken) {
						url.searchParams.set('id_token', authorizeResponse.idToken);
					}
					break;
				}
			}
			if (isTauri()) {
				await openUrl(url.toString());
			} else {
				window.location.href = url.toString();
			}
			break;
		}
		case 'form_post': {
			throw new Error('not supported yet!');
			break;
		}
		case 'web_message': {
			throw new Error('not supported yet!');
			break;
		}
	}
}
