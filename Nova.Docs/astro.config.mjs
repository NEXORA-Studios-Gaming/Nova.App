// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
	integrations: [
		starlight({
			title: 'Nova 文档',
			description: 'Nova Launcher 与组件生态的中文文档。',
			favicon: '/favicon.png',
			logo: {
				src: './src/assets/Nova.png',
				alt: 'Nova',
			},
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/NEXORA-Studios' }],
			sidebar: [
			{
				label: '用户指南',
				items: [{ autogenerate: { directory: 'user' } }],
			},
			{
				label: '开发者指南',
				items: [{ autogenerate: { directory: 'developer' } }],
			},
		],
			components: {
				Sidebar: './src/components/DocsSidebar.astro',
			},
			customCss: ['./src/styles/custom.css'],
		}),
	],
});
