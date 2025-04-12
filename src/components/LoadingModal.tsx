import { Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Card, CardContent } from "@/components/ui/card";

interface LoadingModalProps {
  isOpen: boolean;
  status: string;
  currentDownload?: string;
}

export function LoadingModal({ status, currentDownload }: LoadingModalProps) {
  return (
    <Dialog defaultOpen>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Loading...</DialogTitle>
        </DialogHeader>
        <Card>
          <CardContent className="pt-6">
            <div className="flex flex-col items-center gap-4">
              <div className="flex items-center justify-center">
                <Loader2 className="h-8 w-8 animate-spin text-primary" />
              </div>
              {currentDownload && (
                <p className="text-center text-xs text-muted-foreground">
                  Downloading: {currentDownload}
                </p>
              )}
              <p className="text-center text-xs text-muted-foreground">
                {status}
              </p>
            </div>
          </CardContent>
        </Card>
      </DialogContent>
    </Dialog>
  );
}
