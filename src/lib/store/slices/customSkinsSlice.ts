import { StateCreator } from "zustand";
import { CustomSkin } from "../../types";

export interface CustomSkinsSlice {
    customSkins: Map<number, CustomSkin[]>;
    addCustomSkin: (skin: CustomSkin) => void;
    removeCustomSkin: (skinId: string) => void;
    setCustomSkins: (skins: CustomSkin[]) => void;
}

export const createCustomSkinsSlice: StateCreator<CustomSkinsSlice> = (
    set,
) => ({
    customSkins: new Map(),
    addCustomSkin: (skin) => {
        set((state) => {
            const newCustomSkins = new Map(state.customSkins);
            const championId = skin.champion_id;
            const existingSkins = newCustomSkins.get(championId) ?? [];
            newCustomSkins.set(championId, [...existingSkins, skin]);
            return { customSkins: newCustomSkins };
        });
    },
    removeCustomSkin: (skinId) => {
        set((state) => {
            const newCustomSkins = new Map(state.customSkins);

            for (const [championId, skins] of newCustomSkins.entries()) {
                const updatedSkins = skins.filter((skin) => skin.id !== skinId);

                if (updatedSkins.length !== skins.length) {
                    if (updatedSkins.length === 0) {
                        newCustomSkins.delete(championId);
                    } else {
                        newCustomSkins.set(championId, updatedSkins);
                    }
                    break;
                }
            }

            return { customSkins: newCustomSkins };
        });
    },
    setCustomSkins: (skins) => {
        set(() => {
            const customSkinsMap = new Map<number, CustomSkin[]>();

            skins.forEach((skin) => {
                const championId = skin.champion_id;
                const existingSkins = customSkinsMap.get(championId) ?? [];
                customSkinsMap.set(championId, [...existingSkins, skin]);
            });

            return { customSkins: customSkinsMap };
        });
    },
});
