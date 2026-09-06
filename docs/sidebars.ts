import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    'getting-started',
    {
      type: 'category',
      label: 'Architecture',
      collapsed: false,
      items: [
        'architecture/overview',
        'architecture/three-tier',
        'architecture/crate-structure',
        'architecture/data-flow',
      ],
    },
    {
      type: 'category',
      label: 'Guides',
      collapsed: false,
      items: [
        'guides/native-proxy',
        'guides/browser-proxy',
        'guides/smoldot-integration',
      ],
    },
    {
      type: 'category',
      label: 'Mixnet Protocol',
      items: [
        'protocol/sphinx',
        'protocol/loopix',
        'protocol/surbs',
        'protocol/cover-traffic',
        'protocol/session-management',
      ],
    },
    {
      type: 'category',
      label: 'Operators',
      items: [
        'operators/overview',
      ],
    },
    {
      type: 'category',
      label: 'Security',
      items: [
        'security/threat-model',
      ],
    },
    'comparison',
    'roadmap',
    'timeline',
    'testing',
    'dependencies',
    'faq',
  ],
};

export default sidebars;
