import { useEffect, useRef, useState } from "react"

import { Button } from "@/components/ui/button"
import type { RoomInfo } from "@/lib/bindings/RoomInfo"
import type { ServerMessage } from "@/lib/bindings/ServerMessage"

export function App() {
  const [name, setName] = useState("")
  const [rooms, setRooms] = useState<RoomInfo[]>([])
  const [room, setRoom] = useState<RoomInfo | null>(null)
  const [error, setError] = useState("")
  const ws = useRef<WebSocket | null>(null)

  useEffect(() => {
    if (room) return
    const poll = () =>
      fetch("/rooms")
        .then((r) => r.json())
        .then(setRooms)
        .catch(() => {})
    poll()
    const t = setInterval(poll, 5000)
    return () => clearInterval(t)
  }, [room])

  const join = (id: string) => {
    setError("")
    const proto = location.protocol === "https:" ? "wss" : "ws"
    const sock = new WebSocket(
      `${proto}://${location.host}/ws/${id}?name=${encodeURIComponent(name)}`
    )
    sock.onmessage = (e) => {
      const msg: ServerMessage = JSON.parse(e.data)
      if (msg.type === "joined") setRoom(msg.room)
      else if (msg.type === "player_joined")
        setRoom((r) =>
          r && !r.players.includes(msg.name)
            ? { ...r, players: [...r.players, msg.name] }
            : r
        )
      else if (msg.type === "player_left")
        setRoom((r) => r && { ...r, players: r.players.filter((p) => p !== msg.name) })
      else if (msg.type === "error") setError(msg.message)
    }
    sock.onclose = () => {
      setRoom(null)
      ws.current = null
    }
    ws.current = sock
  }

  const leave = () => ws.current?.close()

  const create = async () => {
    const r = await fetch("/rooms", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ name: `${name}'s game` }),
    })
    const info: RoomInfo = await r.json()
    join(info.id)
  }

  if (room) {
    return (
      <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
        <h1 className="text-lg font-medium">{room.name}</h1>
        <ul className="flex flex-col gap-1 text-sm">
          {room.players.map((p) => (
            <li key={p}>
              {p}
              {p === name && <span className="text-muted-foreground"> (you)</span>}
            </li>
          ))}
        </ul>
        <Button variant="outline" onClick={leave}>
          Leave room
        </Button>
      </div>
    )
  }

  return (
    <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
      <input
        className="rounded-md border bg-transparent px-3 py-2 text-sm"
        placeholder="Your name"
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <Button onClick={create} disabled={!name}>
        Create room
      </Button>
      {error && <p className="text-sm text-red-500">{error}</p>}
      <ul className="flex flex-col gap-2">
        {rooms.map((r) => (
          <li key={r.id} className="flex items-center justify-between rounded-md border p-3 text-sm">
            <span>
              {r.name}
              <span className="text-muted-foreground"> · {r.players.length}/4</span>
            </span>
            <Button
              size="sm"
              variant="outline"
              onClick={() => join(r.id)}
              disabled={!name || r.players.length >= 4}
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

export default App
