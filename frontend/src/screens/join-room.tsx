import { useEffect, useState } from "react"

import { Button } from "@/components/ui/button"
import { getRoom } from "@/lib/api"
import type { RoomInfo } from "@/lib/bindings/RoomInfo"

interface JoinRoomProps {
  roomId: string
  name: string
  setName: (name: string) => void
  onJoin: (roomId: string) => void
  onCancel: () => void
}

export function JoinRoom({ roomId, name, setName, onJoin, onCancel }: JoinRoomProps) {
  const [room, setRoom] = useState<RoomInfo | null | undefined>(undefined)

  useEffect(() => {
    getRoom(roomId).then(setRoom)
  }, [roomId])

  if (room === undefined) {
    return (
      <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
        <p className="text-sm text-muted-foreground">Loading game…</p>
      </div>
    )
  }

  if (room === null) {
    return (
      <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
        <p className="text-sm text-muted-foreground">This game no longer exists.</p>
        <Button variant="outline" onClick={onCancel}>
          Back to lobby
        </Button>
      </div>
    )
  }

  const full = room.players.length >= 4 || room.in_game

  return (
    <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
      <h1 className="text-lg font-medium">{room.name}</h1>
      <p className="text-sm text-muted-foreground">
        {room.players.length}/4 players{room.in_game && " · in progress"}
      </p>
      <input
        className="rounded-md border bg-transparent px-3 py-2 text-sm"
        placeholder="Your name"
        value={name}
        maxLength={32}
        onChange={(e) => setName(e.target.value)}
      />
      <Button onClick={() => onJoin(roomId)} disabled={!name || full}>
        {full ? "Room full" : "Join game"}
      </Button>
      <Button variant="outline" onClick={onCancel}>
        Back to lobby
      </Button>
    </div>
  )
}
