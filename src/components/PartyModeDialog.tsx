import React, { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { toast } from "sonner";
import { Users, UserPlus, UserMinus, RefreshCcw } from "lucide-react";
import { DropdownMenuItem } from "./ui/dropdown-menu";
import {
  partyModeApi,
  getStatusColor,
  getStatusText,
} from "@/lib/api/party-mode";
import type {
  FriendInfo,
  PairedFriend,
  ConnectionRequest as PartyConnectionRequest,
  PairingResponse,
  SkinShare,
} from "@/lib/types/party-mode";

// Types
interface LocalConnectionRequest {
  from_summoner_name: string;
  from_summoner: string;
}

// Friend type - keeping for backward compatibility
interface Friend {
  id: string;
  name: string;
  display_name: string;
  is_online: boolean;
  availability?: string;
  connected_at?: number;
}

// Helper function to format time ago
const formatTimeAgo = (timestamp: number): string => {
  const now = Date.now();
  const diff = now - timestamp;

  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    return `${days} day${days > 1 ? "s" : ""} ago`;
  } else if (hours > 0) {
    return `${hours} hour${hours > 1 ? "s" : ""} ago`;
  } else if (minutes > 0) {
    return `${minutes} minute${minutes > 1 ? "s" : ""} ago`;
  } else {
    return "Just now";
  }
};

export default function PartyModeDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedTab, setSelectedTab] = useState("connect");
  const [searchTerm, setSearchTerm] = useState("");
  const [autoShareEnabled, setAutoShareEnabled] = useState(true);
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [pendingConnectionRequest, setPendingConnectionRequest] =
    useState<LocalConnectionRequest | null>(null);
  const [showConnectionRequest, setShowConnectionRequest] = useState(false);

  // Real data from backend
  const [friends, setFriends] = useState<FriendInfo[]>([]);
  const [pairedFriends, setPairedFriends] = useState<PairedFriend[]>([]);
  const [incomingRequest, setIncomingRequest] =
    useState<PartyConnectionRequest | null>(null);

  // Set up global event listeners that work even when dialog is closed
  useEffect(() => {
    let unsubscribeFunctions: (() => void)[] = [];

    const setupGlobalEventListeners = async () => {
      try {
        // Start chat monitor once on component mount
        await partyModeApi.startChatMonitor();

        // Set up global event listeners that persist even when dialog is closed
        const unsubscribeConnection = await partyModeApi.onConnectionRequest(
          (request) => {
            setIncomingRequest(request);
            setShowConnectionRequest(true);
            setIsOpen(true); // Auto-open the dialog when receiving a connection request
            toast.info(
              `${request.from_summoner_name} wants to connect for skin sharing`
            );
          }
        );

        const unsubscribePairingAccepted = await partyModeApi.onPairingAccepted(
          (response) => {
            toast.success(
              `${response.from_summoner_name} accepted your pairing request!`
            );
            // Reload paired friends regardless of dialog state
            void loadPairedFriends();
          }
        );

        const unsubscribePairingDeclined = await partyModeApi.onPairingDeclined(
          (response) => {
            toast.error(
              `${response.from_summoner_name} declined your pairing request`
            );
          }
        );

        const unsubscribeSkinReceived = await partyModeApi.onSkinReceived(
          (skinShare) => {
            if (notificationsEnabled) {
              toast.info(
                `${skinShare.from_summoner_name} shared ${skinShare.skin_name}`
              );
            }
          }
        );

        // Store unsubscribe functions for cleanup
        unsubscribeFunctions = [
          unsubscribeConnection,
          unsubscribePairingAccepted,
          unsubscribePairingDeclined,
          unsubscribeSkinReceived,
        ];
      } catch (error) {
        console.error("Failed to initialize party mode:", error);
        toast.error("Failed to initialize party mode");
      }
    };

    // Set up global listeners on component mount, not just when dialog opens
    void setupGlobalEventListeners();

    // Return synchronous cleanup function
    return () => {
      unsubscribeFunctions.forEach((unsub) => {
        try {
          unsub();
        } catch (error) {
          console.error("Error during cleanup:", error);
        }
      });
    };
  }, [notificationsEnabled]);

  // Load data when dialog opens
  useEffect(() => {
    if (isOpen) {
      void loadFriends();
      void loadPairedFriends();
    }
  }, [isOpen]);

  const loadFriends = async () => {
    try {
      const friendsList = await partyModeApi.getFriends();
      setFriends(friendsList);
    } catch (error) {
      console.error("Failed to load friends:", error);
      toast.error("Failed to load friends list");
    }
  };

  const loadPairedFriends = async () => {
    try {
      const pairedList = await partyModeApi.getPairedFriends();
      setPairedFriends(pairedList);
    } catch (error) {
      console.error("Failed to load paired friends:", error);
      toast.error("Failed to load paired friends");
    }
  };

  // Helper functions for UI
  const getStatusColorForFriend = (friend: FriendInfo) => {
    return getStatusColor(friend.availability, friend.is_online);
  };

  const getStatusTextForFriend = (friend: FriendInfo) => {
    return getStatusText(friend.availability, friend.is_online);
  };

  const filterFriends = (searchTerm: string) => {
    return friends.filter(
      (friend) =>
        friend.summoner_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        friend.display_name.toLowerCase().includes(searchTerm.toLowerCase())
    );
  };

  // Handlers for UI
  const handleLoadFriendsList = async () => {
    setIsLoading(true);
    try {
      await loadFriends();
    } finally {
      setIsLoading(false);
    }
  };

  const handleConnectWithFriend = async (friendSummonerId: string) => {
    console.log(
      "[DEBUG] handleConnectWithFriend called with friendSummonerId:",
      friendSummonerId
    );
    setIsLoading(true);
    try {
      console.log("[DEBUG] About to call partyModeApi.sendPairingRequest...");
      await partyModeApi.sendPairingRequest(friendSummonerId);
      console.log("[DEBUG] Successfully sent pairing request");
      toast.success("Pairing request sent!");
    } catch (error) {
      console.error("Failed to send pairing request:", error);
      toast.error("Failed to send pairing request");
    } finally {
      setIsLoading(false);
    }
  };

  const handleAcceptConnectionRequest = async () => {
    if (!incomingRequest) return;

    setIsLoading(true);
    try {
      await partyModeApi.respondToPairingRequest(
        "", // request_id not used in current implementation
        incomingRequest.from_summoner_id,
        true
      );
      toast.success("Connection request accepted!");
      setShowConnectionRequest(false);
      setIncomingRequest(null);
      await loadPairedFriends();
    } catch (error) {
      console.error("Failed to accept connection request:", error);
      toast.error("Failed to accept connection request");
    } finally {
      setIsLoading(false);
    }
  };

  const removeFriend = async (friendSummonerId: string) => {
    try {
      await partyModeApi.removePairedFriend(friendSummonerId);
      toast.success("Friend removed successfully");
      await loadPairedFriends();
    } catch (error) {
      console.error("Failed to remove friend:", error);
      toast.error("Failed to remove friend");
    }
  };

  const declineConnectionRequest = async () => {
    if (!incomingRequest) return;

    try {
      await partyModeApi.respondToPairingRequest(
        "", // request_id not used in current implementation
        incomingRequest.from_summoner_id,
        false
      );
      setShowConnectionRequest(false);
      setIncomingRequest(null);
    } catch (error) {
      console.error("Failed to decline connection request:", error);
      toast.error("Failed to decline connection request");
    }
  };

  const toggleAutoShare = async (checked: boolean) => {
    try {
      await partyModeApi.updateSettings(checked, notificationsEnabled);
      setAutoShareEnabled(checked);
    } catch (error) {
      console.error("Failed to update auto share setting:", error);
      toast.error("Failed to update settings");
    }
  };

  const toggleNotifications = async (checked: boolean) => {
    try {
      await partyModeApi.updateSettings(autoShareEnabled, checked);
      setNotificationsEnabled(checked);
    } catch (error) {
      console.error("Failed to update notifications setting:", error);
      toast.error("Failed to update settings");
    }
  };

  const filteredFriends = filterFriends(searchTerm);

  return (
    <>
      <Dialog open={isOpen} onOpenChange={setIsOpen}>
        <DialogTrigger asChild>
          <DropdownMenuItem
            onSelect={(e) => {
              e.preventDefault();
            }}
          >
            <Users className="h-4 w-4" />
            Party Mode
          </DropdownMenuItem>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Users className="h-5 w-5" />
              Party Mode
            </DialogTitle>
          </DialogHeader>

          <Tabs value={selectedTab} onValueChange={setSelectedTab}>
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="connect">Friends</TabsTrigger>
              <TabsTrigger value="connected">
                Connected ({pairedFriends.length})
              </TabsTrigger>
              <TabsTrigger value="settings">Settings</TabsTrigger>
            </TabsList>

            <TabsContent value="connect" className="space-y-4">
              <div className="space-y-4">
                <div className="flex gap-2">
                  <Input
                    placeholder="Search friends..."
                    value={searchTerm}
                    onChange={(e) => {
                      setSearchTerm(e.target.value);
                    }}
                    className="flex-1"
                  />
                  <Button
                    onClick={() => {
                      void handleLoadFriendsList();
                    }}
                    disabled={isLoading}
                    variant={"ghost"}
                    size="icon"
                  >
                    <RefreshCcw className="h-4 w-4" />
                  </Button>
                </div>

                <div className="space-y-2 max-h-96 overflow-y-auto">
                  {isLoading ? (
                    <div className="text-center py-8">
                      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
                      <p className="text-sm text-muted-foreground mt-2">
                        Loading friends...
                      </p>
                    </div>
                  ) : filteredFriends.length === 0 ? (
                    <div className="text-center py-8">
                      <p className="text-muted-foreground">No friends found</p>
                    </div>
                  ) : (
                    filteredFriends.map((friend) => {
                      const isConnected = pairedFriends.some(
                        (cf) => cf.summoner_id === friend.summoner_id
                      );
                      return (
                        <div
                          key={friend.summoner_id}
                          className="flex items-center justify-between p-3 border rounded-lg"
                        >
                          <div className="flex items-center gap-3">
                            <div
                              className={`w-3 h-3 rounded-full ${getStatusColorForFriend(
                                friend
                              )}`}
                            ></div>
                            <div>
                              <p className="font-medium">
                                {friend.display_name}
                              </p>
                              <p className="text-sm text-muted-foreground">
                                {getStatusTextForFriend(friend)}
                              </p>
                            </div>
                          </div>
                          <Button
                            onClick={() => {
                              void handleConnectWithFriend(friend.summoner_id);
                            }}
                            disabled={isConnected || isLoading}
                            size="sm"
                            variant={isConnected ? "secondary" : "default"}
                          >
                            {isConnected ? (
                              <>
                                <UserMinus className="h-4 w-4 mr-2" />
                                Connected
                              </>
                            ) : (
                              <>
                                <UserPlus className="h-4 w-4 mr-2" />
                                Connect
                              </>
                            )}
                          </Button>
                        </div>
                      );
                    })
                  )}
                </div>
              </div>
            </TabsContent>

            <TabsContent value="connected" className="space-y-4">
              <div>
                {pairedFriends.length === 0 ? (
                  <div className="text-center py-8">
                    <Users className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                    <p className="text-muted-foreground">
                      No friends connected
                    </p>
                    <p className="text-sm text-muted-foreground mt-2">
                      Go to the Connect Friends tab to add friends
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {pairedFriends.map((friend) => (
                      <div
                        key={friend.summoner_id}
                        className="flex items-center justify-between p-3 border rounded-lg"
                      >
                        <div className="flex items-center gap-3">
                          <div className="w-3 h-3 rounded-full bg-green-500"></div>
                          <div>
                            <p className="font-medium">{friend.display_name}</p>
                            <p className="text-sm text-muted-foreground">
                              Connected{" "}
                              {new Date(friend.paired_at).toLocaleDateString()}
                            </p>
                          </div>
                        </div>
                        <Button
                          onClick={() => {
                            void removeFriend(friend.summoner_id);
                          }}
                          size="sm"
                          variant="destructive"
                        >
                          <UserMinus className="h-4 w-4 mr-2" />
                          Remove
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </TabsContent>

            <TabsContent value="settings" className="space-y-4">
              <Card>
                <CardContent className="space-y-6">
                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label>Auto-Share Skins</Label>
                      <p className="text-sm text-muted-foreground">
                        Automatically share your skin when you lock in a
                        champion
                      </p>
                    </div>
                    <Switch
                      checked={autoShareEnabled}
                      onCheckedChange={(checked) =>
                        void toggleAutoShare(checked)
                      }
                    />
                  </div>

                  <Separator />

                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label>Notifications</Label>
                      <p className="text-sm text-muted-foreground">
                        Show notifications when skins are shared
                      </p>
                    </div>
                    <Switch
                      checked={notificationsEnabled}
                      onCheckedChange={(checked) =>
                        void toggleNotifications(checked)
                      }
                    />
                  </div>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>

      {/* Connection Request Modal */}
      <Dialog
        open={showConnectionRequest}
        onOpenChange={setShowConnectionRequest}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <UserPlus className="h-5 w-5" />
              Party Mode Connection Request
            </DialogTitle>
          </DialogHeader>

          {incomingRequest && (
            <div className="space-y-4">
              <div className="text-center space-y-2">
                <div className="text-lg font-semibold">
                  {incomingRequest.from_summoner_name ||
                    `User ${incomingRequest.from_summoner_id}`}
                </div>
                <div className="text-sm text-muted-foreground">
                  wants to connect for skin sharing
                </div>
                <div className="text-xs text-muted-foreground">
                  Summoner ID: {incomingRequest.from_summoner_id}
                </div>
                {incomingRequest.timestamp && (
                  <div className="text-xs text-muted-foreground">
                    Sent: {formatTimeAgo(incomingRequest.timestamp)}
                  </div>
                )}
              </div>

              <Separator />

              <div className="text-sm text-center text-muted-foreground">
                Accepting this request will allow you and this friend to share
                skins with each other when you&apos;re both in champion select.
              </div>

              <div className="flex gap-3 justify-center">
                <Button
                  onClick={() => void declineConnectionRequest()}
                  variant="outline"
                  className="flex-1"
                  disabled={isLoading}
                >
                  Decline
                </Button>
                <Button
                  onClick={() => void handleAcceptConnectionRequest()}
                  className="flex-1"
                  disabled={isLoading}
                >
                  {isLoading ? "Connecting..." : "Accept"}
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
