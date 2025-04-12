import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { DataUpdateProgress, DataUpdateResult } from '../types';
import { fetchChampionSummaries, fetchChampionDetails, fetchFantomeFile, transformChampionData } from '../data-utils';

export function useDataUpdate() {
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState<DataUpdateProgress | null>(null);

  const updateData = async (limit: number = 10) => {
    try {
      setIsUpdating(true);
      setProgress({
        currentChampion: '',
        totalChampions: 0,
        processedChampions: 0,
        status: 'checking',
        progress: 0
      });

      // Check for updates
      const updateResult = await invoke<DataUpdateResult>('check_data_updates');
      
      // If no data exists or updates are needed, proceed with update
      if (!updateResult?.updatedChampions || updateResult.updatedChampions.length > 0) {
        // Fetch champion summaries
        const summaries = await fetchChampionSummaries();
        // Limit to first 10 champions
        const limitedSummaries = summaries.slice(0, limit);
        setProgress(prev => ({
          ...prev!,
          totalChampions: limitedSummaries.length,
          status: 'downloading'
        }));

        // Process each champion
        for (let i = 0; i < limitedSummaries.length; i++) {
          const summary = limitedSummaries[i];
          
          // Skip champions with invalid IDs
          if (summary.id <= 0) {
            console.warn(`Skipping champion with invalid ID: ${summary.name} (ID: ${summary.id})`);
            continue;
          }

          setProgress(prev => ({
            ...prev!,
            currentChampion: summary.name,
            processedChampions: i + 1,
            status: 'processing',
            progress: ((i + 1) / limitedSummaries.length) * 100
          }));

          try {
            // Fetch champion details
            const details = await fetchChampionDetails(summary.id);
            
            // Fetch fantome files
            const fantomeFiles = new Map<number, string>();
            for (let skinIndex = 0; skinIndex < details.skins.length; skinIndex++) {
              try {
                const fantomeContent = await fetchFantomeFile(summary.id, skinIndex);
                fantomeFiles.set(skinIndex, fantomeContent);
              } catch (error) {
                console.warn(`Failed to fetch fantome file for ${summary.name} skin ${skinIndex}:`, error);
              }
            }

            // Transform and save data
            const championData = transformChampionData(summary, details, fantomeFiles);
            
            // Validate champion ID
            if (championData.id <= 0) {
              throw new Error(`Invalid champion ID: ${championData.id}`);
            }

            await invoke('update_champion_data', {
              championId: championData.id,
              data: JSON.stringify(championData)
            });

            // Save fantome files
            for (const [skinIndex, content] of fantomeFiles.entries()) {
              await invoke('save_fantome_file', {
                championId: championData.id,
                skinIndex,
                content
              });
            }
          } catch (error) {
            console.error(`Failed to process ${summary.name}:`, error);
            toast.error(`Failed to process ${summary.name}`);
          }
        }

        toast.success(`Data update completed successfully (${limitedSummaries.length} champions)`);
      } else {
        toast.success('Data is up to date');
      }
    } catch (error) {
      console.error('Data update failed:', error);
      toast.error('Failed to update data');
    } finally {
      setIsUpdating(false);
      setProgress(null);
    }
  };

  return {
    isUpdating,
    progress,
    updateData
  };
} 