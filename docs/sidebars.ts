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
      label: 'Zero-Knowledge Proofs',
      items: [
        'zk/stwo-overview',
        'zk/relay-proof',
        'zk/eligibility-proof',
        'zk/tx-validity-proof',
        'zk/cover-compliance-proof',
        'zk/binary-proof-tree',
        'zk/dual-hash-strategy',
      ],
    },
    {
      type: 'category',
      label: 'Smart Contracts (PolkaVM)',
      items: [
        'contracts/registry',
        'contracts/verifier',
        'contracts/revm-adapter',
      ],
    },
    {
      type: 'category',
      label: 'Mixnode Operators',
      items: [
        'operators/overview',
        'operators/validator-mixnodes',
        'operators/standalone-operators',
        'operators/threshold-engine',
        'operators/slashing',
      ],
    },
    {
      type: 'category',
      label: 'API Reference',
      items: [
        'api/builder',
        'api/handle',
        'api/config',
        'api/javascript',
      ],
    },
    {
      type: 'category',
      label: 'Security',
      items: [
        'security/threat-model',
        'security/traffic-analysis',
        'security/trustless-guarantees',
      ],
    },
    {
      type: 'category',
      label: 'Performance',
      items: [
        'performance/benchmarks',
        'performance/latency-analysis',
        'performance/optimization',
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
