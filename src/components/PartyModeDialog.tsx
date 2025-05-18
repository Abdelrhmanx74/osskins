import React, { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogTrigger,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { DropdownMenuItem, DropdownMenuSeparator } from "./ui/dropdown-menu";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Users, UserCheck, UserX, RefreshCw, MailCheck } from "lucide-react";
import { Label } from "./ui/label";
import { Separator } from "./ui/separator";
import { ScrollArea } from "./ui/scroll-area";
import { useSkinSync } from "@/lib/hooks/use-skin-sync";
import {
  AlertDialog,
  AlertDialogDescription,
  AlertDialogTitle,
} from "./ui/alert-dialog";
import { shallow } from "zustand/shallow";
import { usePartyFriends } from "@/lib/hooks/use-party-friends";

interface Friend {
  name: string;
  id: string;
  availability: "online" | "away" | "offline" | "in-game";
  game?: string;
  inParty?: boolean;
}

function PartyMemberList({
  members,
  onRemove,
  getAvailabilityColor,
}: {
  members: { id: string; name: string; availability: string }[];
  onRemove: (id: string) => void;
  getAvailabilityColor: (availability: string) => string;
}) {
  return (
    <>
      <p className="text-sm font-medium">Active Party ({members.length}/5)</p>
      {members.map((member) => (
        <div
          key={member.id}
          className="flex items-center justify-between p-2 rounded-lg bg-primary/10"
        >
          <div className="flex items-center gap-2">
            <div
              className={`h-3 w-3 rounded-full ${getAvailabilityColor(
                member.availability
              )}`}
            ></div>
            <span>{member.name}</span>
          </div>
          <Button
            size="sm"
            variant="destructive"
            onClick={() => onRemove(member.id)}
          >
            Remove
          </Button>
        </div>
      ))}
      <Separator className="my-2" />
    </>
  );
}

function FriendList({
  friends,
  activePartyMembers,
  onSync,
  getAvailabilityColor,
}: {
  friends: { id: string; name: string; availability: string; game?: string }[];
  activePartyMembers: { id: string }[];
  onSync: (id: string, name: string) => void;
  getAvailabilityColor: (availability: string) => string;
}) {
  return (
    <>
      <p className="text-sm font-medium">Available Friends</p>
      {friends
        .filter((friend) => !activePartyMembers.some((m) => m.id === friend.id))
        .map((friend) => (
          <div
            key={friend.id}
            className="flex items-center justify-between p-2 rounded-lg hover:bg-primary/5"
          >
            <div className="flex items-center gap-2">
              <div
                className={`h-3 w-3 rounded-full ${getAvailabilityColor(
                  friend.availability
                )}`}
              ></div>
              <span>{friend.name}</span>
              {friend.game && (
                <span className="text-xs text-muted-foreground ml-1">
                  {friend.game}
                </span>
              )}
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => onSync(friend.id, friend.name)}
            >
              Sync
            </Button>
          </div>
        ))}
    </>
  );
}

export const PartyModeDialog = React.memo(function PartyModeDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [checkingRequests, setCheckingRequests] = useState(false);
  const leaguePath = useGameStore((s) => s.leaguePath);
  const selectedSkins = useGameStore((s) => s.selectedSkins);
  const {
    sendSyncRequest,
    acceptSync,
    rejectSync,
    pendingSyncRequest,
    activePartyMembers,
    checkPendingSyncRequests,
  } = useSkinSync();
  const { friends, isLoading, fetchFriends } = usePartyFriends(leaguePath);

  // Fetch friends when the dialog opens
  useEffect(() => {
    if (isOpen) {
      fetchFriends();
      // Also check for pending sync requests when dialog opens
      handleCheckPendingRequests();
    }
  }, [isOpen]);

  const handleCheckPendingRequests = async () => {
    if (!leaguePath) {
      toast.error(
        "League path not set. Please configure your League of Legends path in settings."
      );
      return;
    }

    setCheckingRequests(true);
    try {
      await checkPendingSyncRequests(true);
      toast.info("Checked for pending sync requests");
    } catch (error) {
      console.error("Error checking for pending requests:", error);
    } finally {
      setCheckingRequests(false);
    }
  };

  const handleSyncRequest = async (friendId: string, friendName: string) => {
    try {
      await sendSyncRequest(friendId, friendName);
      toast.success(`Sync request sent to ${friendName}`);
    } catch (err) {
      console.error(`Failed to send sync request to ${friendName}:`, err);
      toast.error(`Failed to send sync request to ${friendName}`);
    }
  };

  const getAvailabilityColor = (availability: string) => {
    switch (availability) {
      case "online":
        return "bg-green-500";
      case "away":
        return "bg-yellow-500";
      case "in-game":
        return "bg-blue-500";
      default:
        return "bg-gray-500";
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault();
            setIsOpen(true);
          }}
        >
          <Users className="h-4 w-4" />
          Party Mode
        </DropdownMenuItem>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between">
            <span>Party Mode</span>
            <Button
              variant="outline"
              size="sm"
              onClick={handleCheckPendingRequests}
              disabled={checkingRequests || !leaguePath}
            >
              <MailCheck
                className={`h-4 w-4 mr-2 ${
                  checkingRequests ? "animate-spin" : ""
                }`}
              />
              Check Invites
            </Button>
          </DialogTitle>
          <DialogDescription>
            Share your skin selections with friends. When in a party, each
            player will see the others' selected skins.
          </DialogDescription>
        </DialogHeader>

        {pendingSyncRequest && (
          <AlertDialog>
            <AlertDialogTitle>
              Sync Request from{" "}
              {pendingSyncRequest.memberName || pendingSyncRequest.memberId}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {pendingSyncRequest.memberName || pendingSyncRequest.memberId}{" "}
              wants to sync skin selections with you. Accept to see each other's
              skins in game.
            </AlertDialogDescription>
            <div className="flex gap-2 mt-2">
              <Button
                onClick={() =>
                  acceptSync(
                    pendingSyncRequest.memberId,
                    pendingSyncRequest.memberName || pendingSyncRequest.memberId
                  )
                }
                size="sm"
              >
                <UserCheck className="mr-2 h-4 w-4" />
                Accept
              </Button>
              <Button
                onClick={() => rejectSync(pendingSyncRequest.memberId)}
                variant="outline"
                size="sm"
              >
                <UserX className="mr-2 h-4 w-4" />
                Decline
              </Button>
            </div>
          </AlertDialog>
        )}

        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-1 gap-2">
            <div className="flex items-center justify-between">
              <Label>Friends ({friends.length})</Label>
              <Button
                variant="ghost"
                size="sm"
                onClick={fetchFriends}
                disabled={isLoading}
              >
                <RefreshCw
                  className={`h-4 w-4 ${isLoading ? "animate-spin" : ""}`}
                />
              </Button>
            </div>

            <ScrollArea className="h-72 rounded-md border">
              {friends.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
                  {isLoading ? "Loading friends..." : "No friends found"}
                </div>
              ) : (
                <div className="p-4 space-y-2">
                  {activePartyMembers.length > 0 && (
                    <PartyMemberList
                      members={activePartyMembers}
                      onRemove={rejectSync}
                      getAvailabilityColor={getAvailabilityColor}
                    />
                  )}

                  <FriendList
                    friends={friends}
                    activePartyMembers={activePartyMembers}
                    onSync={handleSyncRequest}
                    getAvailabilityColor={getAvailabilityColor}
                  />
                </div>
              )}
            </ScrollArea>
          </div>
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button variant="default">Close</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
});
