import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";

import { FormDialog, TextField } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useApp } from "@/lib/app-state";

/** Danger zone: wipe every property record so a fresh import can start from nothing. */
export function ResetCard() {
  const { api } = useApp();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [confirm, setConfirm] = useState("");

  return (
    <Card className="border-destructive/40">
      <CardHeader>
        <CardTitle>Reset all data</CardTitle>
        <CardDescription>
          Removes every building, unit, tenant, contract, renewal, notice, follow-up, email, document and expense — for starting again after a
          test import. Users, roles, settings, email templates, reminder rules and the audit trail are kept.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Button variant="outline" className="text-destructive" onClick={() => { setConfirm(""); setOpen(true); }}>
          Reset all data…
        </Button>
      </CardContent>
      <FormDialog
        open={open}
        onOpenChange={setOpen}
        title="Reset all data?"
        description="This cannot be undone. All property data is deleted; sign-ins and settings stay."
        submitLabel="Delete everything"
        destructive
        onSubmit={async () => {
          if (confirm.trim() !== "RESET") throw new Error("Type RESET (in capitals) to confirm.");
          await api.resetAllData(confirm.trim());
          await queryClient.invalidateQueries();
          setOpen(false);
          navigate("/");
        }}
      >
        <TextField id="reset-confirm" label="Type RESET to confirm" value={confirm} onChange={setConfirm} autoFocus />
      </FormDialog>
    </Card>
  );
}
