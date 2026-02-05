import { BROWSER } from 'esm-env';

export type ThemeType = 'dark' | 'light';

let theme = $state<ThemeType>(
	BROWSER && window.matchMedia('(prefers-color-scheme: dark)')?.matches ? 'dark' : 'light'
);

export function setTheme(newTheme: ThemeType) {
	theme = newTheme;
}

export function getTheme() {
	return theme;
}

if (BROWSER) {
	if (typeof window !== 'undefined' && window.matchMedia) {
		const mediaQueryList = window.matchMedia('(prefers-color-scheme: dark)');

		function handleColorSchemeChange(event: MediaQueryListEvent | MediaQueryList) {
			theme = event.matches ? 'dark' : 'light';
		}

		mediaQueryList.addEventListener('change', handleColorSchemeChange);

		handleColorSchemeChange(mediaQueryList);
	}
}
