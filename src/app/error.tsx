"use client";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useEffect } from "react";
// import { useRouter } from "next/navigation";
// import Link from "next/link";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("Application error:", error);
  }, [error]);

  // const router = useRouter();

  return (
    <Card className="flex min-h-full flex-col items-center justify-center p-24">
      <CardContent className="flex flex-col items-center justify-center h-full gap-4">
        <div className="text-destructive font-medium">
          Something went wrong!
        </div>
        <div className="text-sm text-muted-foreground">{error.message}</div>
        <div className="flex gap-2 mt-4">
          <Button variant="outline" onClick={reset}>
            Try again
          </Button>
          {/* <Button variant="secondary" onClick={() => router.back()}>
            Back
          </Button> */}
          {/* <Link passHref href="/">
            <Button>Back</Button>
          </Link> */}
        </div>
      </CardContent>
    </Card>
  );
}
