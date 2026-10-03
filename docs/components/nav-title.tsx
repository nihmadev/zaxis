'use client';

import Link from 'next/link';
import type { ComponentProps } from 'react';

export function NavTitle(props: ComponentProps<'a'>) {
  return (
    <>
      <span className="me-auto hidden md:block" aria-hidden="true" />
      <Link {...props} href={props.href ?? '/'} className={`${props.className ?? ''} md:hidden`}>
        <span className="site-title">zaxis <span className="site-version">0.1.0</span></span>
      </Link>
    </>
  );
}
