import { invoke } from "@tauri-apps/api/core";

export interface ManualInjectionSkin {
  champion_id: number;
  skin_id: number;
  chroma_id?: number;
  skin_file?: string;
}

export interface ManualInjectionMiscItem {
  id: string;
  name: string;
  item_type: string;
  skin_file_path: string;
}

export const manualInjectionApi = {
  /**
   * Start manual injection mode with selected skins and misc items.
   * The backend will wait for champion select and then inject.
   */
  async startManualInjection(
    skins: ManualInjectionSkin[],
    miscItems: ManualInjectionMiscItem[],
  ): Promise<void> {
    await invoke("start_manual_injection", {
      skins,
      miscItems,
    });
  },

  /**
   * Stop manual injection mode and clean up any active injections.
   */
  async stopManualInjection(): Promise<void> {
    await invoke("stop_manual_injection");
  },

  /**
   * Set manual injection mode preference (the toggle state).
   * This is separate from starting/stopping a manual injection session.
   */
  async setManualInjectionMode(value: boolean): Promise<void> {
    await invoke("set_manual_injection_mode", { value });
  },

  /**
   * Get manual injection mode preference from backend.
   */
  async getManualInjectionMode(): Promise<boolean> {
    return await invoke("get_manual_injection_mode");
  },
};
