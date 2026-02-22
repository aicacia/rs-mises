import { json } from '@sveltejs/kit';

export const prerender = true;

export async function GET() {
	return json({
		issuer: 'mises://identity',
		authorization_endpoint: 'mises://authorize',
		token_endpoint: 'mises://token',
		userinfo_endpoint: 'mises://user-info',
		jwks_uri: 'mises://jwks.json',
		registration_endpoint: 'mises://register',
		revocation_endpoint: 'mises://revoke',
		introspection_endpoint: 'mises://introspect',
		end_session_endpoint: 'mises://end_session',
		device_authorization_endpoint: 'mises://device_authorize',
		pushed_authorization_request_endpoint: 'mises://pushed_authorize',
		check_session_iframe: 'mises://check_session',
		scopes_supported: ['openid', 'profile', 'email', 'offline_access'],
		response_types_supported: ['code', 'token', 'id_token'],
		response_modes_supported: ['query', 'fragment', 'form_post'],
		grant_types_supported: [
			'authorization_code',
			'refresh_token',
			'client_credentials',
			'urn:ietf:params:oauth:grant-type:device_code'
		],
		token_endpoint_auth_methods_supported: ['none', 'client_secret_basic', 'client_secret_post'],
		token_endpoint_auth_signing_alg_values_supported: ['EdDSA'],
		code_challenge_methods_supported: ['S256'],
		subject_types_supported: ['public'],
		id_token_signing_alg_values_supported: ['EdDSA'],
		userinfo_signing_alg_values_supported: ['EdDSA'],
		request_object_signing_alg_values_supported: ['EdDSA'],
		claims_supported: [
			'iss',
			'aud',
			'exp',
			'jti',
			'scope',
			'acting_for',
			'sub',
			'name',
			'given_name',
			'family_name',
			'preferred_username',
			'email',
			'email_verified',
			'picture'
		],
		claims_parameter_supported: true,
		request_parameter_supported: true,
		request_uri_parameter_supported: false,
		require_request_uri_registration: false,
		frontchannel_logout_supported: false,
		frontchannel_logout_session_supported: false,
		backchannel_logout_supported: false,
		backchannel_logout_session_supported: false
	});
}
