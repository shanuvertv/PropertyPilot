import type { ReactNode } from "react";

import { APP_NAME } from "@/lib/format";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

/** Centered single-card frame shared by Setup, Bootstrap and Login. */
export function AuthFrame({
  title,
  description,
  children,
  footer,
}: {
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/40 px-6 py-10">
      <div className="w-full max-w-[400px]">
        <div className="mb-5 text-center">
          <div className="text-[15px] font-semibold tracking-tight">{APP_NAME}</div>
          <div className="text-[12px] text-muted-foreground">Rental contract renewal management</div>
        </div>
        <Card>
          <CardHeader>
            <CardTitle>{title}</CardTitle>
            {description && <CardDescription>{description}</CardDescription>}
          </CardHeader>
          <CardContent>{children}</CardContent>
        </Card>
        {footer && <div className="mt-4 text-center text-[12.5px] text-muted-foreground">{footer}</div>}
      </div>
    </div>
  );
}
