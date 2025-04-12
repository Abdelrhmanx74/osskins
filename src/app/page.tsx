'use client';

import { useEffect } from 'react';
import { useDataUpdate } from '@/lib/hooks/use-data-update';
import { DataUpdateModal } from '@/components/DataUpdateModal';
import { Toaster } from 'sonner';
import { useRouter } from 'next/navigation';

export default function Home() {
  const { isUpdating, progress, updateData } = useDataUpdate();
  const router = useRouter();

  useEffect(() => {
    let mounted = true;

    async function initialize() {
      try {
        // Limit to first 10 champions for initial load
        await updateData(10);
        if (mounted) {
          router.push('/champions');
        }
      } catch (error) {
        console.error('Failed to initialize:', error);
      }
    }

    void initialize();

    return () => {
      mounted = false;
    };
  }, []); // Empty dependency array means this runs once on mount

  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24">
      <DataUpdateModal isOpen={isUpdating} progress={progress} />
      <Toaster />
    </main>
  );
}
