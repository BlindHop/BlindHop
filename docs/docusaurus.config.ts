import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';

const config: Config = {
  title: 'BlindHop',
  tagline: 'Mixnet Privacy Layer for Smoldot Light Clients',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://wiki.blindhop.wtf',
  baseUrl: '/',

  organizationName: 'blindhop',
  projectName: 'docs',

  onBrokenLinks: 'warn',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  markdown: {
    mermaid: true,
  },

  themes: ['@docusaurus/theme-mermaid'],

  stylesheets: [
    {
      href: 'https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css',
      type: 'text/css',
      integrity: 'sha384-nB0miv6/jRmo5OSEAFP2mCigf6JEwUMmOW7woHFWJGxPh5brY1cloudflare',
      crossorigin: 'anonymous',
    },
  ],

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: '/',
          editUrl: 'https://github.com/blindhop/blindhop/tree/HEAD/docs/',
          remarkPlugins: [remarkMath],
          rehypePlugins: [rehypeKatex],
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/blindhop-social-card.png',
    colorMode: {
      defaultMode: 'dark',
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'BlindHop',
      logo: {
        alt: 'BlindHop Logo',
        src: 'img/logo.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Documentation',
        },
        {
          href: 'https://blindhop.wtf',
          label: 'Website',
          position: 'right',
        },
        {

          href: 'https://github.com/blindhop/blindhop',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Documentation',
          items: [
            { label: 'Getting Started', to: '/getting-started' },
            { label: 'Architecture', to: '/architecture/overview' },
            { label: 'Guides', to: '/guides/native-proxy' },
          ],
        },
        {
          title: 'Protocol',
          items: [
            { label: 'Sphinx Protocol', to: '/protocol/sphinx' },
            { label: 'Loopix Cover Traffic', to: '/protocol/loopix' },
            { label: 'Threat Model', to: '/security/threat-model' },
          ],
        },
        {
          title: 'Community',
          items: [
            { label: 'GitHub', href: 'https://github.com/blindhop/blindhop' },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} BlindHop Contributors. Apache 2.0 + MIT.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml', 'bash', 'json', 'solidity'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
