import type { Metadata } from 'next';
import type { ReactNode } from 'react';
import { Provider } from './provider';
import './global.css';

export const metadata: Metadata = {
  title: { default: 'zaxis documentation', template: '%s · zaxis' },
  description: 'Desktop GUI in Rust. Installation, widgets, input, repaint scheduling, rendering, and the drawing protocol.',
};

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
