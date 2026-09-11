import type { ReactNode } from "react";

export function PageHeader({
  title,
  description,
  actions,
}: {
  title: string;
  description?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="mb-4 flex flex-wrap items-start justify-between gap-x-4 gap-y-3 md:mb-6">
      <div className="min-w-0">
        <h1 className="text-[20px] font-semibold tracking-tight text-balance md:text-[22px]">{title}</h1>
        {description && <p className="mt-1 max-w-[68ch] text-[13.5px] text-muted-foreground">{description}</p>}
      </div>
      {actions && <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>}
    </div>
  );
}
