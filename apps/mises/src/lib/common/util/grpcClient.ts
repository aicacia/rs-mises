import { ConfigurationServiceDefinition, OidcServiceDefinition } from '$lib/proto/mises';
import { channel } from './channel';
import { once } from './once';
import { createClientFactory, Metadata, type ClientMiddleware } from 'nice-grpc-web';

let authorizationToken: string | null = null;

export function setAuthorizationToken(accessToken: string | null): void {
	authorizationToken = accessToken;
}

const authMiddleware: ClientMiddleware = async function* (call, options) {
	if (authorizationToken) {
		options.metadata = new Metadata(options.metadata);
		options.metadata.set('authorization', `Bearer ${authorizationToken}`);
	}
	return yield* call.next(call.request, options);
};

const clientFactory = once(() => createClientFactory().use(authMiddleware));

export const configurationClient = once(() =>
	clientFactory().create(ConfigurationServiceDefinition, channel())
);
export const oidcClient = once(() => clientFactory().create(OidcServiceDefinition, channel()));
