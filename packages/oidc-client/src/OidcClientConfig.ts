import { OpenIdConfigurationJSON } from './OpenIdConfigurationJSON.js';

export type OidcClientConfig = {
	/**
	 * Base URL of the OIDC provider (issuer).
	 */
	authority: string;
	/**
	 * Optional explicit OIDC configuration endpoint URL. If not set, it's constructed from the authority.
	 */
	configurationUrl?: string;
	/**
	 * Cached or preloaded provider configuration JSON.
	 */
	configuration?: Partial<OpenIdConfigurationJSON>;
	/**
	 * Initial configuration used to seed a fetch when network is unavailable.
	 */
	configurationSeed?: Partial<OpenIdConfigurationJSON>;
	/**
	 * JWKS signing keys provided by the provider.
	 */
	signingKeys?: Record<string, unknown>[];
	/**
	 * Client identifier issued by the provider.
	 */
	clientId?: string;
	/**
	 * Client secret for confidential clients.
	 */
	clientSecret?: string;
	/**
	 * Expected response_type for authorization requests (e.g. "code", "id_token", "token").
	 */
	responseType?: string;
	/**
	 * Requested OAuth2 scopes (space‑delimited).
	 */
	scope?: string;
	/**
	 * Redirect URI where the provider will send responses.
	 */
	redirectUri: string;
	/**
	 * URI to return users after logout.
	 */
	postLogoutRedirectUri?: string;
	/**
	 * Method used to authenticate the client at the token endpoint.
	 */
	clientAuthentication?: 'client_secret_basic' | 'client_secret_post' | 'client_secret_jwt';
	/**
	 * Algorithm for signing client authentication JWTs if used.
	 */
	tokenEndpointAuthSigningAlg?: 'HS256' | 'HS384' | 'HS512';
	/**
	 * Prompt parameter to control UI (login, consent, etc.).
	 */
	prompt?: string;
	/**
	 * Display hint for the authorization UI.
	 */
	display?: string;
	/**
	 * Maximum allowed authentication age in seconds.
	 */
	maxAge?: number;
	/**
	 * Desired UI languages.
	 */
	uiLocales?: string;
	/**
	 * Requested ACR values.
	 */
	acrValues?: string;
	/**
	 * Resource or resources being requested.
	 */
	resource?: string | string[];
	/**
	 * How the authorization response is returned (query or fragment).
	 */
	responseMode?: 'query' | 'fragment';
	/**
	 * Whether to filter out standard protocol claims from the id_token.
	 */
	filterProtocolClaims?: boolean | string[];
	/**
	 * Load userinfo endpoint after authentication.
	 */
	loadUserInfo?: boolean;
	/**
	 * Maximum age of a state object before considering it stale.
	 */
	staleStateAgeInSeconds?: number;
	/**
	 * Strategy for merging array claims when combining metadata/keys.
	 */
	mergeClaimsStrategy?: { array: 'replace' | 'merge' };
	/**
	 * Custom storage for state objects.
	 */
	stateStore?: unknown;
	/**
	 * Additional query parameters for authorization requests.
	 */
	extraQueryParams?: Record<string, string | number | boolean>;
	/**
	 * Additional parameters for token requests.
	 */
	extraTokenParams?: Record<string, unknown>;
	/**
	 * Extra headers to include on HTTP requests.
	 */
	extraHeaders?: Record<string, string | (() => string)>;
	/**
	 * DPoP key material or configuration.
	 */
	dpop?: unknown;
	/**
	 * Additional Content-Type values for token revocation.
	 */
	revokeTokenAdditionalContentTypes?: string[];
	/**
	 * Disable PKCE for the authorization request.
	 */
	disablePkce?: boolean;
	/**
	 * Credentials option passed to fetch calls.
	 */
	fetchRequestCredentials?: RequestCredentials;
	/**
	 * Scope that is allowed for refresh tokens.
	 */
	refreshTokenAllowedScope?: string;
	/**
	 * Timeout for network requests in seconds.
	 */
	requestTimeoutInSeconds?: number;
	/**
	 * Omit the scope parameter when making requests.
	 */
	omitScopeWhenRequesting?: boolean;
};
