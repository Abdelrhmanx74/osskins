"use client";

import { useEffect, useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { CustomSkin } from "@/lib/types";
import { slugify } from "@/lib/utils/slugify";

interface CustomSkinRenameDialogProps {
    skin: CustomSkin | null;
    open: boolean;
    onClose: () => void;
    onSave: (name: string) => Promise<void>;
    saving?: boolean;
}

export function CustomSkinRenameDialog({ skin, open, onClose, onSave, saving = false }: CustomSkinRenameDialogProps) {
    const [name, setName] = useState("");

    useEffect(() => {
        if (skin) {
            setName(skin.name);
        } else {
            setName("");
        }
    }, [skin]);

    const handleSave = async () => {
        if (!skin) return;
        await onSave(name.trim());
    };

    return (
        <Dialog open={open} onOpenChange={(o) => { if (!o) onClose(); }}>
            <DialogContent className="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>Set custom skin name</DialogTitle>
                </DialogHeader>
                <div className="flex flex-col gap-3">
                    <div className="flex flex-col gap-2">
                        <Label htmlFor="custom-skin-name">Display name</Label>
                        <Input
                            id="custom-skin-name"
                            value={name}
                            onChange={(e) => setName(e.target.value)}
                            placeholder="e.g. Star Guardian Lux"
                        />
                    </div>
                </div>
                <DialogFooter>
                    <Button variant="outline" onClick={onClose} disabled={saving}>Cancel</Button>
                    <Button onClick={handleSave} disabled={!name.trim() || saving}>
                        {saving ? "Saving..." : "Save"}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
