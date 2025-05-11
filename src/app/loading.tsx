"use client";

import { Loader2 } from "lucide-react";

export default function Loading() {
  return (
    <div className="flex items-center justify-center h-screen w-full flex-col gap-4 animate-in fade-in-50 duration-500">
      <Loader2 className="h-12 w-12 animate-spin" />
    </div>
  );
}
