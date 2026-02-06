import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Mindia Docs',
  base: '/mindia/', // required for GitHub Pages project site (https://<user>.github.io/mindia/)
  ignoreDeadLinks: true,
  description:
    'High-performance media management API. Upload and manage images, videos, documents, and audio with S3 storage, on-the-fly transformations, HLS streaming, and semantic search.',
  appearance: 'dark',
  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Quick Start', link: '/quick-start' },
      { text: 'API Reference', link: '/api-reference' },
    ],
    socialLinks: [
      {
        icon: 'github',
        link: 'https://github.com/jeremybastin1207/mindia',
      },
    ],
    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Quick Start', link: '/quick-start' },
          { text: 'Installation', link: '/installation' },
          { text: 'Configuration', link: '/configuration' },
        ],
      },
      {
        text: 'Authentication & Security',
        items: [
          { text: 'Authentication', link: '/authentication' },
          { text: 'API Keys', link: '/api-keys' },
          { text: 'Multi-Tenancy', link: '/multi-tenancy' },
          { text: 'Authorization', link: '/authorization' },
        ],
      },
      {
        text: 'Core Features',
        items: [
          { text: 'Images', link: '/images' },
          { text: 'Image Transformations', link: '/image-transformations' },
          { text: 'Videos', link: '/videos' },
          { text: 'Audio', link: '/audio' },
          { text: 'Documents', link: '/documents' },
          { text: 'Folders', link: '/folders' },
          { text: 'File Groups', link: '/file-groups' },
        ],
      },
      {
        text: 'Advanced Features',
        items: [
          { text: 'Plugins', link: '/plugins' },
          { text: 'Tasks', link: '/tasks' },
          { text: 'Semantic Search', link: '/semantic-search' },
          { text: 'Webhooks', link: '/webhooks' },
          { text: 'Analytics', link: '/analytics' },
        ],
      },
      {
        text: 'Integration',
        items: [
          { text: 'Client Integration', link: '/client-integration' },
          { text: 'Error Handling', link: '/error-handling' },
          { text: 'Rate Limiting', link: '/rate-limiting' },
          { text: 'Best Practices', link: '/best-practices' },
          { text: 'API Reference', link: '/api-reference' },
        ],
      },
    ],
    search: {
      provider: 'local',
    },
  },
})
