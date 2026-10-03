'use client';

import { RootProvider } from 'fumadocs-ui/provider/next';
import type { ReactNode } from 'react';

export function Provider({ children }: { children: ReactNode }) {
  return (
    <RootProvider
      search={{ options: {
        type: 'static',
        api: `${process.env.NEXT_PUBLIC_BASE_PATH ?? ''}/search-index.json`,
      } }}
    >
      {children}
    </RootProvider>
  );
}
