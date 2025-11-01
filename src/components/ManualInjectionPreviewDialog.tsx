"use client";

import { useGameStore, type MiscItemType } from "@/lib/store";
import { useChampions } from "@/lib/hooks/use-champions";
import { useMiscItems } from "@/lib/hooks/use-misc-items";
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Card, CardContent, CardFooter } from "@/components/ui/card";
import { useI18n } from "@/lib/i18n";
import { X, Map, Languages, Shapes, Package, BadgeX } from "lucide-react";
import { Button } from "./ui/button";
import Image from "next/image";
import { useMemo } from "react";

interface ManualInjectionPreviewDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

export function ManualInjectionPreviewDialog({
    open,
    onOpenChange,
}: ManualInjectionPreviewDialogProps) {
    const { t } = useI18n();
    const { champions } = useChampions();
    const { miscItems } = useMiscItems();
    const manualSelectedSkins = useGameStore((state) => state.manualSelectedSkins);
    const selectedMiscItems = useGameStore((state) => state.selectedMiscItems);
    const clearManualSelection = useGameStore((state) => state.clearManualSelection);
    const toggleMiscItemSelection = useGameStore((state) => state.toggleMiscItemSelection);
    const selectMiscItem = useGameStore((state) => state.selectMiscItem);

    // Built-in fonts (same as in MiscItemView.tsx) - these are static files in resources/fonts
    const builtinFonts = useMemo(() => [
        {
            id: "builtin-font-chinese",
            name: "Chinese",
            item_type: "font",
            skin_file_path: "chinese.skin_file",
        },
        {
            id: "builtin-font-korean",
            name: "Korean",
            item_type: "font",
            skin_file_path: "korean.skin_file",
        },
        {
            id: "builtin-font-minecraft",
            name: "Minecraft",
            item_type: "font",
            skin_file_path: "minecraft.skin_file",
        },
        {
            id: "builtin-font-arcade",
            name: "Arcade",
            item_type: "font",
            skin_file_path: "arcade.skin_file",
        },
    ], []);

    // Get selected skins with champion and skin details
    const selectedSkinsDetails = useMemo(() => {
        const details: Array<{
            championId: number;
            championName: string;
            championIcon: string;
            skinId: number;
            skinName: string;
            skinImage: string;
            chromaId?: number;
            chromaName?: string;
        }> = [];

        manualSelectedSkins.forEach((selection, championId) => {
            const champion = champions.find((c) => c.id === championId);
            if (!champion) return;

            const skin = champion.skins.find((s) => s.id === selection.skinId);
            if (!skin) return;

            let chromaName: string | undefined;
            if (selection.chromaId) {
                const chroma = skin.chromas.find((c) => c.id === selection.chromaId);
                chromaName = chroma?.name;
            }

            details.push({
                championId: champion.id,
                championName: champion.name,
                championIcon: champion.iconSrc,
                skinId: skin.id,
                skinName: skin.name,
                skinImage: skin.skinSrc,
                chromaId: selection.chromaId,
                chromaName,
            });
        });

        return details;
    }, [manualSelectedSkins, champions]);

    // Get selected misc items with details
    const selectedMiscDetails = useMemo(() => {
        const details: Array<{
            id: string;
            name: string;
            type: string;
            miscType: MiscItemType;
            icon: typeof Map;
        }> = [];

        selectedMiscItems.forEach((itemIds, type) => {
            // Combine built-in fonts with uploaded items for fonts, otherwise just uploaded items
            const typeItems = miscItems.get(type) ?? [];
            const allItems = type === "font" ? [...builtinFonts, ...typeItems] : typeItems;

            itemIds.forEach((itemId) => {
                const item = allItems.find((i) => i.id === itemId);

                if (!item) {
                    console.warn(`Item not found for id: ${itemId} in type: ${type}`);
                    return;
                }

                // Determine icon based on type
                let icon = Package;
                if (type === "map") icon = Map;
                else if (type === "font") icon = Languages;
                else if (type === "hud") icon = Shapes;

                details.push({
                    id: item.id,
                    name: item.name,
                    type: item.item_type,
                    miscType: type,
                    icon,
                });
            });
        });

        return details;
    }, [selectedMiscItems, miscItems, builtinFonts]);

    const totalItems = selectedSkinsDetails.length + selectedMiscDetails.length;

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-5xl sm:max-h-[80vh] overflow-hidden overflow-y-auto">
                <DialogHeader>
                    <DialogTitle className="text-xl font-bold">
                        {t("manual_injection.preview_title")} ({totalItems})
                    </DialogTitle>
                </DialogHeader>

                {/* Single Grid with Misc Items First, Then Skins */}
                {totalItems > 0 ? (
                    <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4 h-full p-1 overflow-hidden overflow-y-auto">
                        {/* Render Misc Items First */}
                        {selectedMiscDetails.map((detail) => {
                            const Icon = detail.icon;

                            const handleMiscRemove = () => {
                                // For misc type (multi-select), toggle off the item
                                if (detail.miscType === "misc") {
                                    toggleMiscItemSelection(detail.miscType, detail.id);
                                } else {
                                    // For other types (single-select), clear the selection
                                    selectMiscItem(detail.miscType, null);
                                }
                            };

                            return (
                                <Card
                                    key={detail.id}
                                    className="relative rounded-2xl overflow-hidden group ring-2 ring-primary transition-all p-0 min-h-[420px]"
                                >
                                    <CardContent className="p-0 h-full w-full relative">
                                        {/* Icon background */}
                                        <div className="relative w-full h-full min-h-[420px] bg-gradient-to-br from-muted/50 to-muted flex items-center justify-center">
                                            <Icon className="h-20 w-20 text-muted-foreground/40" strokeWidth={1.5} />
                                        </div>

                                        {/* Remove button - top right */}
                                        <Button
                                            size="icon"
                                            variant="destructive"
                                            className="absolute top-2 right-2 h-8 w-8 opacity-0 group-hover:opacity-100 transition-all duration-200 z-30 rounded-full shadow-lg hover:scale-110"
                                            onClick={(e) => {
                                                e.stopPropagation();
                                                handleMiscRemove();
                                            }}
                                        >
                                            <X className="h-4 w-4" />
                                        </Button>

                                        {/* Bottom overlay with item info */}
                                        <CardFooter className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/95 via-black/75 to-transparent p-3 flex flex-col gap-1 z-20">
                                            <p className="text-sm font-bold text-white truncate drop-shadow-md w-full">
                                                {detail.name}
                                            </p>
                                            <p className="text-xs text-white/70 truncate drop-shadow-md w-full capitalize">
                                                {detail.type}
                                            </p>
                                        </CardFooter>
                                    </CardContent>
                                </Card>
                            );
                        })}

                        {/* Render Skins After Misc Items */}
                        {selectedSkinsDetails.map((detail) => (
                            <Card
                                key={detail.championId}
                                className="relative rounded-2xl overflow-hidden group ring-2 ring-primary transition-all p-0 min-h-[420px]"
                            >
                                <CardContent className="p-0 h-full w-full relative">
                                    {/* Skin splash art background */}
                                    <div className="relative w-full h-full">
                                        <Image
                                            src={detail.skinImage}
                                            alt={detail.skinName}
                                            fill
                                            className="object-cover"
                                        />
                                    </div>

                                    {/* Remove button - top right with better UI */}
                                    <Button
                                        size="icon"
                                        variant="destructive"
                                        className="absolute top-2 right-2 h-8 w-8 opacity-0 group-hover:opacity-100 transition-all duration-200 z-30 rounded-full shadow-lg hover:scale-110"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            clearManualSelection(detail.championId);
                                        }}
                                    >
                                        <X className="h-4 w-4" />
                                    </Button>

                                    {/* Bottom overlay with champion info */}
                                    <CardFooter className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/95 via-black/75 to-transparent p-3 flex flex-col gap-2 z-20">
                                        {/* Champion icon + name */}
                                        <div className="flex justify-start self-start items-center gap-2">
                                            <div className="relative w-8 h-8 flex-shrink-0 overflow-hidden">
                                                <Image
                                                    src={detail.championIcon}
                                                    alt={detail.championName}
                                                    fill
                                                    className="object-cover"
                                                />
                                            </div>
                                            <p className="text-white drop-shadow-md">
                                                {detail.skinName}
                                            </p>
                                        </div>

                                        {/* Chroma name if exists */}
                                        {detail.chromaName && (
                                            <p className="text-xs text-primary font-semibold truncate drop-shadow-md">
                                                {detail.chromaName}
                                            </p>
                                        )}
                                    </CardFooter>
                                </CardContent>
                            </Card>
                        ))}
                    </div>
                ) : (
                    /* Empty State */
                    <div className="text-center py-16 text-muted-foreground">
                        <BadgeX className="h-16 w-16 mx-auto mb-4 opacity-20" />
                        <p className="text-lg">{t("manual_injection.no_items_selected")}</p>
                    </div>
                )}
            </DialogContent>
        </Dialog>
    );
}
