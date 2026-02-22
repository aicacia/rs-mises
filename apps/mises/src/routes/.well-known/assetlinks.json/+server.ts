import { json } from '@sveltejs/kit';
import { PUBLIC_CERT_FINGERPRINT } from '$env/static/public';

export const prerender = true;

export async function GET() {
	return json([
		{
			relation: ['delegate_permission/common.handle_all_urls'],
			target: {
				namespace: 'com.mises',
				package_name: 'com.mises',
				sha256_cert_fingerprints: [PUBLIC_CERT_FINGERPRINT]
			}
		}
	]);
}
