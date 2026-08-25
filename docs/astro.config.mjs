import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://ax2.github.io',
  base: '/zifile',
  integrations: [
    starlight({
      title: 'ZiFile 文档',
      description: 'ZiFile 产品、架构、安全、开发与发布文档',
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/ax2/zifile',
        },
      ],
      locales: {
        root: {
          label: '简体中文',
          lang: 'zh-CN',
        },
        en: {
          label: 'English',
          lang: 'en',
        },
      },
      sidebar: [
        {
          label: '概览',
          translations: { en: 'Overview' },
          items: [
            { label: '项目首页', translations: { en: 'Home' }, slug: '' },
            { label: '格式计划', translations: { en: 'Format plan' }, slug: 'formats' },
          ],
        },
        {
          label: '产品',
          translations: { en: 'Product' },
          items: [{ autogenerate: { directory: 'product' } }],
        },
        {
          label: '架构',
          translations: { en: 'Architecture' },
          items: [{ autogenerate: { directory: 'architecture' } }],
        },
        {
          label: '开发',
          translations: { en: 'Development' },
          items: [{ autogenerate: { directory: 'development' } }],
        },
        {
          label: '阶段记录',
          translations: { en: 'Release records' },
          items: [{ autogenerate: { directory: 'releases' } }],
        },
      ],
    }),
  ],
});
