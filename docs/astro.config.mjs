import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://zifile.zicode.com',
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
      },
      sidebar: [
        { label: '概览', items: [{ label: '项目首页', slug: '' }, { label: '格式计划', slug: 'formats' }] },
        { label: '产品', items: [{ autogenerate: { directory: 'product' } }] },
        { label: '架构', items: [{ autogenerate: { directory: 'architecture' } }] },
        { label: '开发', items: [{ autogenerate: { directory: 'development' } }] },
        { label: '阶段记录', items: [{ autogenerate: { directory: 'releases' } }] },
      ],
    }),
  ],
});
