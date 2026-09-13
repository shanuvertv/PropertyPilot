import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Wand2 } from "lucide-react";

import type { Cheque, Contract } from "@/api/types-domain";
import { DataTable, useViewMode, ViewToggle } from "@/components/DataTable";
import { errorMessage } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { useApp } from "@/lib/app-state";
import { formatMoney } from "@/lib/format";
import { ChequeDialog, GenerateChequesDialog, chequeColumns } from "./ChequeParts";

/** The contract's post-dated cheques: set them up from the rent, then track each deposit. */
export function ContractCheques({ contract }: { contract: Contract }) {
  const { api, can } = useApp();
  const queryClient = useQueryClient();
  const manage = can("MANAGE_CONTRACTS");
  const list = useQuery({ queryKey: ["cheques", "contract", contract.id], queryFn: () => api.contractCheques(contract.id) });
  const [view, setView] = useViewMode("contract-cheques");
  const [add, setAdd] = useState(false);
  const [generate, setGenerate] = useState(false);
  const [edit, setEdit] = useState<Cheque | null>(null);
  const rows = list.data ?? [];
  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["cheques"] });
    setAdd(false);
    setGenerate(false);
    setEdit(null);
  };

  const total = rows.reduce((a, c) => a + (c.status === "CANCELLED" ? 0 : c.amount), 0);
  const cleared = rows.filter((c) => c.status === "CLEARED").reduce((a, c) => a + c.amount, 0);
  const pending = rows.filter((c) => c.status === "PENDING");
  const overdue = pending.filter((c) => c.daysUntilDue < 0).length;
  const next = pending.filter((c) => c.daysUntilDue >= 0).sort((a, b) => a.daysUntilDue - b.daysUntilDue)[0];
  const columns = chequeColumns({ manage, showContract: false, onEdit: (c) => setEdit(c) });

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-[13px] text-muted-foreground">
          {rows.length === 0 ? (
            "No cheques recorded for this contract."
          ) : (
            <>
              {rows.length} cheque{rows.length === 1 ? "" : "s"} · {formatMoney(total)} total · {formatMoney(cleared)} cleared
              {contract.rentAmount !== null && Math.abs(total - contract.rentAmount) >= 0.005 && <span className="text-amber-600"> · differs from the rent ({formatMoney(contract.rentAmount)})</span>}
              {overdue > 0 && <span className="text-destructive"> · {overdue} overdue</span>}
              {next && <span> · next: {next.chequeNumber ?? `cheque ${next.seq}`} {next.daysUntilDue === 0 ? "today" : `in ${next.daysUntilDue} days`}</span>}
            </>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <ViewToggle value={view} onChange={setView} />
          {manage && (
            <>
              <Button size="sm" variant="outline" onClick={() => setGenerate(true)}>
                <Wand2 data-icon="inline-start" />
                Set up cheques
              </Button>
              <Button size="sm" onClick={() => setAdd(true)}>
                <Plus data-icon="inline-start" />
                Add cheque
              </Button>
            </>
          )}
        </div>
      </div>
      <DataTable view={view} columns={columns} rows={list.data} rowKey={(c) => c.id} loading={list.isPending} error={list.isError ? errorMessage(list.error, "Could not load the cheques.") : null} empty={manage ? "Use “Set up cheques” to split the rent into post-dated cheques, or add them one by one." : "No cheques recorded."} />
      <ChequeDialog open={add || edit !== null} onOpenChange={(o) => { if (!o) { setAdd(false); setEdit(null); } }} contractId={contract.id} edit={edit} onSaved={refresh} />
      <GenerateChequesDialog open={generate} onOpenChange={setGenerate} contractId={contract.id} rentAmount={contract.rentAmount} startDate={contract.startDate} hasPending={pending.length > 0} onSaved={refresh} />
    </div>
  );
}
