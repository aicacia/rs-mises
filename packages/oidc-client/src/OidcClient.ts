import { EventEmitter } from 'eventemitter3';
import type { OidcClientConfig } from './OidcClientConfig.js';
import type { OpenIdConfigurationJSON } from './OpenIdConfigurationJSON.js';
import { openUrl } from './util/openUrl.js';
import { createFetchWithTimeout, type Fetch } from './util/createFetchWithTimeout.js';
import { generateState } from './util/generateState.js';
import { snakeCase } from './util/snakeCase.js';
import { nativeFetch } from './NativeFetch.js';

export type OidcClientEvents = {
	error: (error: unknown) => void;
};

export type OidcClientOptions = {
	clientConfig: OidcClientConfig;
	fetch?: Fetch;
};

export type AuthorizationUrlOptions = {
	state?: string;
	extraParams?: Record<string, string | number | boolean>;
};

export type RegistrationOptions = {
	redirectUri?: string;
	registration?: Record<string, unknown>;
	popup?: boolean;
	windowFeatures?: string;
};

type DynamicClientRegistrationResponse = {
	client_id?: string;
	client_secret?: string;
};

const OPTIONAL_AUTHORIZATION_PARAMS = [
	'prompt',
	'display',
	'maxAge',
	'uiLocales',
	'acrValues',
	'resource',
	'responseMode'
] as const;

export type SigninOptions = AuthorizationUrlOptions & {
	popup?: boolean;
	windowFeatures?: string;
};

export class OidcClient extends EventEmitter<OidcClientEvents> {
	private readonly config: OidcClientConfig;
	private readonly fetch: Fetch;

	private openIdConfigPromise: Promise<OpenIdConfigurationJSON> | null = null;

	constructor(options: OidcClientOptions) {
		super();
		this.config = OidcClient.validateConfig(options.clientConfig);
		this.fetch = this.config.requestTimeoutInSeconds
			? createFetchWithTimeout(this.config.requestTimeoutInSeconds * 1000, options.fetch)
			: (options.fetch ?? fetch);
	}

	setClientId(clientId: string, clientSecret?: string): void {
		this.config.clientId = clientId;
		if (clientSecret) {
			this.config.clientSecret = clientSecret;
		}
	}

	handleRegistrationCallback(url?: string | URL): string | null {
		const currentUrl =
			typeof url === 'string'
				? new URL(url)
				: (url ?? (typeof window !== 'undefined' ? new URL(window.location.href) : null));
		if (!currentUrl) {
			return null;
		}

		const clientId = currentUrl.searchParams.get('client_id');
		if (!clientId) {
			return null;
		}

		const clientSecret = currentUrl.searchParams.get('client_secret');
		this.setClientId(clientId, clientSecret ?? undefined);
		return clientId;
	}

	static validateConfig(config: OidcClientConfig): OidcClientConfig {
		if (!config.authority || typeof config.authority !== 'string') {
			throw new Error('Invalid authority URL');
		}
		config.authority = config.authority.endsWith('/')
			? config.authority.slice(0, -1)
			: config.authority;
		if (!config.redirectUri || typeof config.redirectUri !== 'string') {
			throw new Error('Invalid redirectUri');
		}
		return config;
	}

	private async fetchOpenIdConfiguration(): Promise<OpenIdConfigurationJSON> {
		const url = `${this.config.authority}/.well-known/openid-configuration`;
		const urlObj = new URL(url);
		const isNative = urlObj.protocol !== 'http:' && urlObj.protocol !== 'https:';

		if (isNative) {
			const res = await nativeFetch<OpenIdConfigurationJSON>(urlObj);
			return res;
		}

		const res = await this.fetch(url, {
			credentials: this.config.fetchRequestCredentials ?? 'same-origin'
		});
		if (!res.ok) {
			throw new Error(`Failed to fetch OIDC configuration: ${res.status} ${res.statusText}`);
		}
		return res.json() as Promise<OpenIdConfigurationJSON>;
	}

	async getOpenIdConfiguration(): Promise<OpenIdConfigurationJSON> {
		if (!this.openIdConfigPromise) {
			this.openIdConfigPromise = this.fetchOpenIdConfiguration();
		}
		return this.openIdConfigPromise;
	}

	async signin(options: SigninOptions = {}): Promise<URL> {
		const url = await this.getAuthorizationUrl(options);
		await openUrl(url, {
			popup: options.popup,
			windowFeatures: options.windowFeatures
		});
		return url;
	}

	async getRegistrationUrl(options: RegistrationOptions = {}): Promise<URL> {
		const config = await this.getOpenIdConfiguration();
		if (!config.registration_endpoint) {
			throw new Error('Provider does not support dynamic client registration');
		}

		const url = new URL(config.registration_endpoint);
		url.searchParams.set('redirect_uri', options.redirectUri ?? this.config.redirectUri);
		if (options.registration) {
			url.searchParams.set('registration', JSON.stringify(options.registration));
		}
		return url;
	}

	async startRegistration(options: RegistrationOptions = {}): Promise<URL> {
		const url = await this.getRegistrationUrl(options);
		await openUrl(url, {
			popup: options.popup,
			windowFeatures: options.windowFeatures
		});
		return url;
	}

	private async registerClient(config: OpenIdConfigurationJSON): Promise<string> {
		const endpoint = config.registration_endpoint as string;
		const endpointUrl = new URL(endpoint);
		const isNative = endpointUrl.protocol !== 'http:' && endpointUrl.protocol !== 'https:';

		if (isNative) {
			endpointUrl.searchParams.set('redirect_uris', JSON.stringify([this.config.redirectUri]));
			const response = await nativeFetch<DynamicClientRegistrationResponse>(endpointUrl);
			if (!response.client_id) {
				throw new Error('registration response missing client_id');
			}
			this.setClientId(response.client_id, response.client_secret);
			return response.client_id;
		}

		const res = await this.fetch(endpoint, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				redirect_uris: [this.config.redirectUri]
			})
		});
		if (!res.ok) {
			throw new Error(`Failed dynamic client registration: ${res.status}`);
		}

		const json = (await res.json()) as DynamicClientRegistrationResponse;
		if (!json.client_id) {
			throw new Error('registration response missing client_id');
		}

		this.setClientId(json.client_id, json.client_secret);
		return json.client_id;
	}

	private async getClientId(config: OpenIdConfigurationJSON): Promise<string> {
		if (this.config.clientId) {
			return this.config.clientId;
		}
		if (!config.registration_endpoint) {
			throw new Error('clientId is required for signin');
		}

		if (typeof window === 'undefined') {
			return this.registerClient(config);
		}

		throw new Error(
			'clientId is required for signin; call startRegistration/getRegistrationUrl and then setClientId or handleRegistrationCallback'
		);
	}

	async getAuthorizationUrl(options: AuthorizationUrlOptions = {}): Promise<URL> {
		const config = await this.getOpenIdConfiguration();
		const clientId = await this.getClientId(config);
		const url = new URL(config.authorization_endpoint);

		url.searchParams.set('client_id', clientId);
		url.searchParams.set('redirect_uri', this.config.redirectUri);
		url.searchParams.set('response_type', this.config.responseType ?? 'code');

		if (!this.config.omitScopeWhenRequesting) {
			url.searchParams.set('scope', this.config.scope ?? 'openid');
		}

		const state = options.state ?? generateState();
		url.searchParams.set('state', state);

		for (const key of OPTIONAL_AUTHORIZATION_PARAMS) {
			const value = this.config[key];
			if (value !== undefined && value !== null) {
				url.searchParams.set(snakeCase(key), String(value));
			}
		}

		if (this.config.extraQueryParams) {
			for (const [k, v] of Object.entries(this.config.extraQueryParams)) {
				url.searchParams.set(k, String(v));
			}
		}

		if (options.extraParams) {
			for (const [k, v] of Object.entries(options.extraParams)) {
				url.searchParams.set(k, String(v));
			}
		}

		return url;
	}
}
