// Ersatz für react-virtuoso in Tests: jsdom misst keine Größen, die echte Liste
// würde nichts zeigen. Der Ersatz rendert alle Einträge und bietet dieselben Handles.
import { useImperativeHandle, type ReactNode, type Ref } from "react";
import { vi } from "vitest";

export const handles = { scrollIntoView: vi.fn(), scrollToIndex: vi.fn() };

interface ListProps<T> {
  ref?: Ref<unknown>;
  data: T[];
  itemContent: (i: number, t: T) => ReactNode;
  scrollerRef?: (el: HTMLElement | null) => void;
  className?: string;
}
export function Virtuoso<T>({ ref, data, itemContent, scrollerRef, className }: ListProps<T>) {
  useImperativeHandle(ref, () => handles);
  return (
    <div className={className} ref={(el) => scrollerRef?.(el)} data-testid="liste">
      {data.map((t, i) => (
        <div key={i}>{itemContent(i, t)}</div>
      ))}
    </div>
  );
}

interface GridProps {
  ref?: Ref<unknown>;
  totalCount: number;
  itemContent: (i: number) => ReactNode;
  scrollerRef?: (el: HTMLElement | Window | null) => void;
  className?: string;
  rangeChanged?: (r: { startIndex: number; endIndex: number }) => void;
}
export function VirtuosoGrid({ ref, totalCount, itemContent, scrollerRef, className }: GridProps) {
  useImperativeHandle(ref, () => handles);
  return (
    <div className={className} ref={(el) => scrollerRef?.(el)} data-testid="raster">
      {Array.from({ length: totalCount }, (_, i) => (
        <div key={i}>{itemContent(i)}</div>
      ))}
    </div>
  );
}
