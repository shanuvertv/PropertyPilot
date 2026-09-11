import { useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, FileText, Trash2, Upload } from "lucide-react";

import type { DocumentEntity } from "@/api/types-domain";
import { errorMessage } from "@/components/forms";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { formatBytes, formatDateTime } from "@/lib/format";

/** Fetches the bytes with the bearer token and hands the file to the browser/webview. */
export async function saveBlob(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

export function DocumentsPanel({ entityType, entityId, canManage }: { entityType: DocumentEntity; entityId: string; canManage: boolean }) {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const key = ["documents", entityType, entityId];
  const docs = useQuery({ queryKey: key, queryFn: () => api.listDocuments(entityType, entityId) });
  const [error, setError] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  const upload = useMutation({
    mutationFn: (file: File) => api.uploadDocument(entityType, entityId, file),
    onSuccess: () => {
      setError(null);
      void queryClient.invalidateQueries({ queryKey: key });
    },
    onError: (e) => setError(errorMessage(e, "Upload failed.")),
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteDocument(id),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: key }),
    onError: (e) => setError(errorMessage(e, "Could not remove the document.")),
  });

  async function download(id: string, fileName: string) {
    try {
      const blob = await api.downloadDocument(id);
      await saveBlob(blob, fileName);
    } catch (e) {
      setError(errorMessage(e, "Download failed."));
    }
  }

  return (
    <div className="flex flex-col gap-3">
      {error && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
      {canManage && (
        <div className="flex items-center gap-2">
          <input
            ref={fileInput}
            type="file"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) upload.mutate(f);
              e.target.value = "";
            }}
          />
          <Button variant="outline" size="sm" onClick={() => fileInput.current?.click()} disabled={upload.isPending}>
            <Upload data-icon="inline-start" />
            {upload.isPending ? "Uploading…" : "Upload document"}
          </Button>
          <span className="text-[12px] text-muted-foreground">Up to 25 MB per file.</span>
        </div>
      )}
      <ul className="divide-y rounded-md border bg-card">
        {docs.data?.length === 0 && <li className="px-4 py-6 text-center text-[13px] text-muted-foreground">No documents yet.</li>}
        {docs.data?.map((d) => (
          <li key={d.id} className="flex items-center gap-3 px-4 py-2.5">
            <FileText className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13.5px] font-medium">{d.fileName}</div>
              <div className="text-[12px] text-muted-foreground">
                {formatBytes(d.sizeBytes)} · {d.uploadedByName ?? "—"} · {formatDateTime(d.createdAt)}
              </div>
            </div>
            <Button variant="ghost" size="icon-sm" title="Download" aria-label="Download" onClick={() => void download(d.id, d.fileName)}>
              <Download />
            </Button>
            {canManage && (
              <Button
                variant="ghost"
                size="icon-sm"
                title="Remove"
                aria-label="Remove"
                onClick={() => {
                  if (window.confirm(`Remove ${d.fileName}?`)) remove.mutate(d.id);
                }}
              >
                <Trash2 />
              </Button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
