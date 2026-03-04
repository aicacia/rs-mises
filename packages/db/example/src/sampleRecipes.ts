import { recipesCollection, type Recipe } from './collections/recipes.js';

const sampleRecipes: Omit<Recipe, 'id' | 'createdAt' | 'updatedAt'>[] = [
	{
		title: 'Classic Chocolate Chip Cookies',
		description: 'Soft and chewy chocolate chip cookies with a perfect golden edge.',
		ingredients: [
			{
				item: { name: 'Butter', description: 'Unsalted butter, softened' },
				quantity: { value: 113, unit: 'g' }
			},
			{
				item: { name: 'Brown Sugar', description: 'Packed brown sugar' },
				quantity: { value: 200, unit: 'g' }
			},
			{
				item: { name: 'White Sugar', description: 'Granulated sugar' },
				quantity: { value: 100, unit: 'g' }
			},
			{ item: { name: 'Eggs', description: 'Large eggs' }, quantity: { value: 2, unit: 'g' } },
			{
				item: { name: 'Vanilla Extract', description: 'Pure vanilla extract' },
				quantity: { value: 10, unit: 'ml' }
			},
			{
				item: { name: 'All-Purpose Flour', description: 'Sifted flour' },
				quantity: { value: 280, unit: 'g' }
			},
			{
				item: { name: 'Baking Soda', description: 'Baking soda' },
				quantity: { value: 5, unit: 'g' }
			},
			{ item: { name: 'Salt', description: 'Fine salt' }, quantity: { value: 5, unit: 'g' } },
			{
				item: { name: 'Chocolate Chips', description: 'Semi-sweet chocolate chips' },
				quantity: { value: 340, unit: 'g' }
			}
		],
		instructions: [
			'Preheat oven to 375°F (190°C).',
			'In a large bowl, cream together butter and both sugars until light and fluffy.',
			'Beat in eggs one at a time, then stir in vanilla extract.',
			'In a separate bowl, whisk together flour, baking soda, and salt.',
			'Gradually blend the dry ingredients into the creamed mixture.',
			'Fold in chocolate chips.',
			'Drop rounded tablespoons of dough onto ungreased cookie sheets.',
			'Bake for 9-11 minutes or until golden brown.',
			'Cool on baking sheet for 2 minutes before removing to a wire rack.'
		]
	},
	{
		title: 'Simple Tomato Basil Pasta',
		description: 'A quick and fresh pasta dish with cherry tomatoes and basil.',
		ingredients: [
			{
				item: { name: 'Spaghetti', description: 'Dry pasta' },
				quantity: { value: 400, unit: 'g' }
			},
			{
				item: { name: 'Cherry Tomatoes', description: 'Fresh, halved' },
				quantity: { value: 500, unit: 'g' }
			},
			{
				item: { name: 'Garlic', description: 'Minced cloves' },
				quantity: { value: 15, unit: 'g' }
			},
			{
				item: { name: 'Olive Oil', description: 'Extra virgin' },
				quantity: { value: 60, unit: 'ml' }
			},
			{
				item: { name: 'Fresh Basil', description: 'Chopped leaves' },
				quantity: { value: 20, unit: 'g' }
			},
			{
				item: { name: 'Parmesan', description: 'Grated cheese' },
				quantity: { value: 50, unit: 'g' }
			},
			{ item: { name: 'Salt', description: 'To taste' }, quantity: { value: 5, unit: 'g' } }
		],
		instructions: [
			'Bring a large pot of salted water to boil and cook spaghetti according to package directions.',
			'Heat olive oil in a large pan over medium heat.',
			'Add minced garlic and sauté for 1 minute until fragrant.',
			'Add cherry tomatoes and cook for 5-7 minutes until they start to burst.',
			'Season with salt and pepper.',
			'Drain pasta, reserving 1 cup of pasta water.',
			'Add pasta to the tomato mixture and toss to combine.',
			'Add pasta water as needed to create a light sauce.',
			'Remove from heat and stir in fresh basil.',
			'Serve topped with grated Parmesan.'
		]
	},
	{
		title: 'Classic Pancakes',
		description: 'Fluffy American-style pancakes perfect for breakfast.',
		ingredients: [
			{
				item: { name: 'All-Purpose Flour', description: 'Sifted' },
				quantity: { value: 200, unit: 'g' }
			},
			{
				item: { name: 'Sugar', description: 'White granulated' },
				quantity: { value: 25, unit: 'g' }
			},
			{
				item: { name: 'Baking Powder', description: 'Fresh baking powder' },
				quantity: { value: 10, unit: 'g' }
			},
			{ item: { name: 'Salt', description: 'Fine salt' }, quantity: { value: 3, unit: 'g' } },
			{ item: { name: 'Milk', description: 'Whole milk' }, quantity: { value: 300, unit: 'ml' } },
			{ item: { name: 'Egg', description: 'Large egg' }, quantity: { value: 1, unit: 'g' } },
			{ item: { name: 'Butter', description: 'Melted butter' }, quantity: { value: 45, unit: 'g' } }
		],
		instructions: [
			'In a large bowl, whisk together flour, sugar, baking powder, and salt.',
			'In another bowl, whisk together milk, egg, and melted butter.',
			'Pour wet ingredients into dry ingredients and stir until just combined (batter should be slightly lumpy).',
			'Heat a griddle or non-stick pan over medium heat and lightly grease.',
			'Pour 1/4 cup of batter for each pancake onto the griddle.',
			'Cook until bubbles form on the surface and edges look set, about 2-3 minutes.',
			'Flip and cook until golden brown on the other side, about 1-2 minutes.',
			'Serve warm with maple syrup and butter.'
		]
	}
];

export async function populateSampleRecipes(): Promise<void> {
	for (const sampleRecipe of sampleRecipes) {
		await recipesCollection.create({
			...sampleRecipe,
			id: Math.random().toString(36).substring(2),
			createdAt: new Date(),
			updatedAt: new Date()
		});
		await new Promise((resolve) => setTimeout(resolve, 1000)); // Simulate async delay
	}
}
