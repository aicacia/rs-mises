import { ConfigurationServiceDefinition, OidcServiceDefinition } from '$lib/proto/mises';
import { channel } from './channel';
import { once } from './once';
import { createClient } from 'nice-grpc-web';

export const configurationClient = once(() =>
	createClient(ConfigurationServiceDefinition, channel())
);
export const oidcClient = once(() => createClient(OidcServiceDefinition, channel()));
