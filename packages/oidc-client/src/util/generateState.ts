export function generateState(): string {
	try {
		const arr = new Uint8Array(16);
		if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
			crypto.getRandomValues(arr);
		} else {
			for (let i = 0; i < arr.length; i++) {
				arr[i] = Math.floor(Math.random() * 256);
			}
		}
		return Array.from(arr)
			.map((b) => b.toString(16).padStart(2, '0'))
			.join('');
	} catch {
		return Math.random().toString(36).substring(2);
	}
}
