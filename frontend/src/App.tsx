import { useState } from "react"

import { useGameSocket } from "@/hooks/use-game-socket"
import { roomIdFromUrl, setRoomUrl } from "@/lib/route"
import { JoinRoom } from "@/screens/join-room"
import { Lobby } from "@/screens/lobby"
import { RoomScreen } from "@/screens/room"

export function App() {
  const [name, setName] = useState("")
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
