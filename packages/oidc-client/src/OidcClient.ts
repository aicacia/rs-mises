import { EventEmitter } from 'eventemitter3';
import type { OidcClientConfig } from './OidcClientConfig.js';
import type { OpenIdConfigurationJSON } from './OpenIdConfigurationJSON.js';
import { createFetchWithTimeout, Fetch } from './util/createFetchWithTimeout.js';
import { generateState } from './util/generateState.js';
import { snakeCase } from './util/snakeCase.js';

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

	async signin(options: SigninOptions = {}): Promise<[window: Window | null, url: URL]> {
		const url = await this.getAuthorizationUrl(options);

		if (options.popup) {
			let childWindow: Window | null = null;

			if (typeof window !== 'undefined' && typeof window.open === 'function') {
				childWindow = window.open(url, '_blank', options.windowFeatures ?? '');
			}
			return [childWindow, url];
		} else {
			if (typeof window !== 'undefined' && window.location) {
				window.location.assign(url);
			}
			return [null, url];
		}
	}

	private async getClientId(config: OpenIdConfigurationJSON): Promise<string> {
		if (this.config.clientId) {
			return this.config.clientId;
		}

		if (!config.registration_endpoint) {
			throw new Error('clientId is required for signin');
		}

		// if running in browser, perform an interactive registration
		// by navigating to the provider's registration endpoint. the
		// caller is expected to handle the redirect and retry after a
		// client_id has been returned in the query string.
		if (typeof window !== 'undefined') {
			const url = new URL(config.registration_endpoint);
			url.searchParams.set('redirect_uri', window.location.href);
			window.location.assign(url.toString());
			// never resolves because we navigated away
			return new Promise(() => {});
		}

		// fallback for non-browser environments: do a POST
		const res = await this.fetch(config.registration_endpoint, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				redirect_uris: [this.config.redirectUri]
			})
		});
		if (!res.ok) {
			throw new Error(`Failed dynamic client registration: ${res.status}`);
		}
		const json = await res.json();
		if (!json.client_id) {
			throw new Error('registration response missing client_id');
		}
		this.config.clientId = json.client_id;
		if (json.client_secret) {
			this.config.clientSecret = json.client_secret;
		}
		return json.client_id;
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
