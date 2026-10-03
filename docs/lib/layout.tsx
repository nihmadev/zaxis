import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { NavTitle } from '@/components/nav-title';

export const repository = 'https://github.com/nihmadev/zaxis';

export const layoutOptions: BaseLayoutProps = {
  nav: {
    title: NavTitle,
    url: '/',
  },
  githubUrl: repository,
};
