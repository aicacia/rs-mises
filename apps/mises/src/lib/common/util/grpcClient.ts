import { BootstrapServiceDefinition, OidcServiceDefinition } from '$lib/proto/mises';
import { channel } from './channel';
import { once } from './once';
import { createClient } from 'nice-grpc-web';

export const bootstrapClient = once(() => createClient(BootstrapServiceDefinition, channel()));
export const oidcClient = once(() => createClient(OidcServiceDefinition, channel()));
