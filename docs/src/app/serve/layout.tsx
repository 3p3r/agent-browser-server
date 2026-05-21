import { pageMetadata } from "@/lib/page-titles";

export const metadata = pageMetadata("serve");

export default function ServeLayout({ children }: { children: React.ReactNode }) {
  return children;
}
