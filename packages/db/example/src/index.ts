import { mount } from 'svelte';
import App from './App.svelte';
import { populateSampleRecipes } from './sampleRecipes.js';

populateSampleRecipes().catch((error) => {
	console.error('Error populating sample recipes:', error);
});

mount(App, {
	target: document.body
});
