import { useState } from "react"

import { Button } from "@/components/ui/button"
import { useGameSocket } from "@/hooks/use-game-socket"
import { isKnownPath, roomIdFromUrl, setRoomUrl } from "@/lib/route"
import { JoinRoom } from "@/screens/join-room"
import { Lobby } from "@/screens/lobby"
import { RoomScreen } from "@/screens/room"

export function App() {
  const [name, setName] = useState("")
  const [notFound] = useState(() => !isKnownPath())
  const [pendingRoomId, setPendingRoomId] = useState(roomIdFromUrl)
  const { room, game, join, leave, send } = useGameSocket(name)

  const joinRoom = (id: string) => {
    setRoomUrl(id)
    setPendingRoomId(id)
    join(id)
  }

  const backToLobby = () => {
    setRoomUrl(null)
    setPendingRoomId(null)
  }

  if (notFound) {
    return (
      <div className="mx-auto flex min-h-svh max-w-md flex-col gap-4 p-6">
        <h1 className="text-lg font-medium">404 — Page not found</h1>
        <Button variant="outline" onClick={() => (window.location.href = "/")}>
          Back to lobby
        </Button>
      </div>
    )
  }

  if (room) {
    return (
      <RoomScreen
        name={name}
        room={room}
        game={game}
        send={send}
        leave={() => {
          leave()
          backToLobby()
        }}
      />
    )
  }

  if (pendingRoomId) {
    return (
      <JoinRoom
        key={pendingRoomId}
        roomId={pendingRoomId}
        name={name}
        setName={setName}
        onJoin={joinRoom}
        onCancel={backToLobby}
      />
    )
  }

  return <Lobby name={name} setName={setName} onJoin={joinRoom} />
}

export default App
