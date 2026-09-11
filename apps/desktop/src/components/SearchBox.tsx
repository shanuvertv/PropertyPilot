import { useEffect, useState } from "react";
import { Search, X } from "lucide-react";

import { Input } from "@/components/ui/input";

/** Debounced search input; `onChange` fires 300 ms after typing stops. */
export function SearchBox({
  value,
  onChange,
  placeholder,
  className,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  className?: string;
}) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  useEffect(() => {
    if (text === value) return;
    const t = setTimeout(() => onChange(text), 300);
    return () => clearTimeout(t);
  }, [text, value, onChange]);

  return (
    <div className={`relative w-full max-w-[360px] ${className ?? ""}`}>
      <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" aria-hidden="true" />
      <Input
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={placeholder ?? "Search"}
        className="pl-8 pr-8"
        aria-label={placeholder ?? "Search"}
      />
      {text && (
        <button
          type="button"
          className="absolute top-1/2 right-2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          onClick={() => {
            setText("");
            onChange("");
          }}
          aria-label="Clear search"
        >
          <X className="size-3.5" />
        </button>
      )}
    </div>
  );
}
