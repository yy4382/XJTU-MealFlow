//  @ts-check

import { tanstackConfig } from '@tanstack/eslint-config'

export default [
  {
    ignores: ['node_modules', 'dist', '*.config.js'],
  },
  ...tanstackConfig,
  {
    rules: {
      '@typescript-eslint/array-type': [
        'error',
        { default: 'array', readonly: 'array' },
      ],
      'import/consistent-type-specifier-style': 'off',
      'sort-imports': 'off',
      'import/order': 'off',
    },
  },
]
