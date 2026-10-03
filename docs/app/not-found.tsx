import Link from 'next/link';

export default function NotFound() {
  return (
    <main className="m-auto max-w-lg p-8">
      <h1 className="mb-4 text-2xl font-semibold">Page not found</h1>
      <Link className="text-fd-primary underline" href="/">Open the documentation index</Link>
    </main>
  );
}
