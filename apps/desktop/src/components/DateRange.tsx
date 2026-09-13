import { Input } from "@/components/ui/input";

/** From/to date pair used by list filters; either side may be empty. */
export function DateRange({ from, to, onChange }: { from?: string; to?: string; onChange: (from: string | undefined, to: string | undefined) => void }) {
  return (
    <div className="flex items-center gap-1.5 text-[12.5px] text-muted-foreground">
      <Input type="date" value={from ?? ""} onChange={(e) => onChange(e.target.value || undefined, to)} className="w-[9.5rem]" aria-label="From date" />
      <span>to</span>
      <Input type="date" value={to ?? ""} onChange={(e) => onChange(from, e.target.value || undefined)} className="w-[9.5rem]" aria-label="To date" />
    </div>
  );
}
