"use client";

import Image from "next/image";
import React from "react";

export default function Splash() {
  return (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center">
      <div className="flex flex-col items-center gap-6">
        <div className="relative">
          <Image
            src="/Square310x310Logo.png"
            alt="app logo"
            width={160}
            height={40}
            className="animate-pulse"
          />
        </div>
      </div>
    </div>
  );
}
