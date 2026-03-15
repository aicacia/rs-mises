import test from 'tape';
import { OidcClient } from './OidcClient.js';
import type { OidcConfiguration } from './OidcConfiguration.js';
import { OidcClientError } from './OidcClientError.js';

function createClient(): OidcClient {
	return new OidcClient({
		clientConfig: {
			authority: 'https://issuer.example',
			registration: {
				redirectUris: ['https://app.example/callback']
			}
		}
	});
}

function createOidcConfiguration(overrides: Partial<OidcConfiguration> = {}): OidcConfiguration {
	return {
		issuer: 'https://issuer.example',
		authorization_endpoint: 'https://issuer.example/oauth/authorize',
		token_endpoint: 'https://issuer.example/oauth/token',
		check_session_iframe: 'https://issuer.example/oauth/session',
		end_session_endpoint: 'https://issuer.example/oauth/logout',
		jwks_uri: 'https://issuer.example/oauth/jwks',
		response_types_supported: ['code'],
		subject_types_supported: ['public'],
		...overrides
	};
}

test('getUserInfo throws NO_ACCESS_TOKEN when token is missing', async (t) => {
	const client = createClient();

	try {
		await client.getUserInfo();
		t.fail('Expected getUserInfo to throw');
	} catch (error) {
		t.ok(error instanceof OidcClientError, 'Expected OidcClientError');
		if (error instanceof OidcClientError) {
			t.equal(error.code, 'NO_ACCESS_TOKEN', 'Should expose NO_ACCESS_TOKEN reason code');
		}
	}

	t.end();
});

test('getUserInfo throws NO_USERINFO_ENDPOINT when provider does not expose endpoint', async (t) => {
	const client = createClient();
	const testClient = client as unknown as {
		getStoredTokenResponse: () => { access_token?: string } | null;
		getOidcConfiguration: () => Promise<OidcConfiguration>;
	};

	testClient.getStoredTokenResponse = () => ({ access_token: 'access-token' });
	testClient.getOidcConfiguration = async () =>
		createOidcConfiguration({
			userinfo_endpoint: undefined
		});

	try {
		await client.getUserInfo();
		t.fail('Expected getUserInfo to throw');
	} catch (error) {
		t.ok(error instanceof OidcClientError, 'Expected OidcClientError');
		if (error instanceof OidcClientError) {
			t.equal(error.code, 'NO_USERINFO_ENDPOINT', 'Should expose NO_USERINFO_ENDPOINT reason code');
		}
	}

	t.end();
});
