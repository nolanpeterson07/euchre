import { useState } from "react"

import { useGameSocket } from "@/hooks/use-game-socket"
import { Lobby } from "@/screens/lobby"
import { RoomScreen } from "@/screens/room"

export function App() {
  const [name, setName] = useState("")
  const { room, game, join, leave, send } = useGameSocket(name)

  if (room) {
    return <RoomScreen name={name} room={room} game={game} send={send} leave={leave} />
  }
  return <Lobby name={name} setName={setName} onJoin={join} />
}

export default App
