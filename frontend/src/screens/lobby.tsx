import { useEffect, useState } from "react"

import { Button } from "@/components/ui/button"
import { createRoom, listRooms } from "@/lib/api"
import type { RoomInfo } from "@/lib/bindings/RoomInfo"

interface LobbyProps {
  name: string
  setName: (name: string) => void
  onJoin: (roomId: string) => void
}

export function Lobby({ name, setName, onJoin }: LobbyProps) {
  const [rooms, setRooms] = useState<RoomInfo[]>([])

  useEffect(() => {
    const poll = () => listRooms().then(setRooms).catch(() => {})
    poll()
    const t = setInterval(poll, 5000)
    return () => clearInterval(t)
  }, [])

  const create = async () => {
    const info = await createRoom(`${name}'s game`)
    onJoin(info.id)
  }

  return (
    <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
      <h1 className="text-lg font-medium">Euchre</h1>
      <input
        className="rounded-md border bg-transparent px-3 py-2 text-sm"
        placeholder="Your name"
        value={name}
        maxLength={32}
        onChange={(e) => setName(e.target.value)}
      />
      <Button onClick={create} disabled={!name}>
        Create room
      </Button>
      <ul className="flex flex-col gap-2">
        {rooms.map((r) => (
          <li
            key={r.id}
            className="flex items-center justify-between rounded-md border p-3 text-sm"
          >
            <span>
              {r.name}
              <span className="text-muted-foreground"> · {r.players.length}/4</span>
            </span>
            <Button
              size="sm"
              variant="outline"
              onClick={() => onJoin(r.id)}
              disabled={!name || r.players.length >= 4 || r.in_game}
            >
              Join
            </Button>
          </li>
        ))}
        {rooms.length === 0 && (
          <li className="text-sm text-muted-foreground">No open games.</li>
        )}
      </ul>
    </div>
  )
}
