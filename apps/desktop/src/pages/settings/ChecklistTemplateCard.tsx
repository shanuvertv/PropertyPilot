import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, Plus, X } from "lucide-react";

import type { ChecklistTemplateItem } from "@/api/types-domain";
import { errorMessage } from "@/components/forms";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { useApp } from "@/lib/app-state";

/** Spec §13: Admin customises the renewal completion checklist. */
export function ChecklistTemplateCard() {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const template = useQuery({ queryKey: ["checklist-template"], queryFn: () => api.checklistTemplate() });
  const [items, setItems] = useState<ChecklistTemplateItem[]>([]);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (template.data && !dirty) setItems(template.data.filter((i) => i.active));
  }, [template.data, dirty]);

  const save = useMutation({
    mutationFn: () => api.saveChecklistTemplate(items.map((i, idx) => ({ ...i, active: true, label: i.label.trim() || `Item ${idx + 1}` }))),
    onSuccess: () => {
      setDirty(false);
      setSaved(true);
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["checklist-template"] });
      setTimeout(() => setSaved(false), 2500);
    },
    onError: (e) => setError(errorMessage(e)),
  });

  function patch(idx: number, p: Partial<ChecklistTemplateItem>) {
    setItems((list) => list.map((i, k) => (k === idx ? { ...i, ...p } : i)));
    setDirty(true);
  }
  function move(idx: number, delta: number) {
    setItems((list) => {
      const next = [...list];
      const j = idx + delta;
      if (j < 0 || j >= next.length) return list;
      [next[idx], next[j]] = [next[j], next[idx]];
      return next;
    });
    setDirty(true);
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Renewal checklist</CardTitle>
        <CardDescription>Copied into every new renewal case. Required items must be ticked before a renewal can be completed. Items with a system key are ticked automatically.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {error && (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}
        <ul className="flex flex-col gap-1.5">
          {items.map((i, idx) => (
            <li key={i.id ?? `new-${idx}`} className="flex items-center gap-2">
              <span className="w-5 text-right text-[12px] tabular-nums text-muted-foreground">{idx + 1}</span>
              <Input value={i.label} onChange={(e) => patch(idx, { label: e.target.value })} aria-label={`Item ${idx + 1}`} />
              <label className="flex items-center gap-1.5 text-[12.5px] whitespace-nowrap">
                <Checkbox checked={i.required} onCheckedChange={(c) => patch(idx, { required: c === true })} />
                Required
              </label>
              {i.key && <span className="rounded bg-muted px-1.5 font-mono text-[10px] text-muted-foreground" title="Ticked automatically by the system">auto</span>}
              <Button size="icon-xs" variant="ghost" onClick={() => move(idx, -1)} disabled={idx === 0} aria-label="Move up">
                <ArrowUp />
              </Button>
              <Button size="icon-xs" variant="ghost" onClick={() => move(idx, 1)} disabled={idx === items.length - 1} aria-label="Move down">
                <ArrowDown />
              </Button>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label="Remove"
                onClick={() => {
                  setItems((list) => list.filter((_, k) => k !== idx));
                  setDirty(true);
                }}
              >
                <X />
              </Button>
            </li>
          ))}
        </ul>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              setItems((list) => [...list, { id: null, key: null, label: "", required: false, active: true }]);
              setDirty(true);
            }}
          >
            <Plus data-icon="inline-start" />
            Add item
          </Button>
          <Button size="sm" onClick={() => save.mutate()} disabled={!dirty || save.isPending}>
            {save.isPending ? "Saving…" : "Save checklist"}
          </Button>
          {saved && <span className="text-[12.5px] text-muted-foreground">Saved.</span>}
        </div>
      </CardContent>
    </Card>
  );
}
