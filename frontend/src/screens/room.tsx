import { Button } from "@/components/ui/button"
import type { ClientMessage } from "@/lib/bindings/ClientMessage"
import type { RoomInfo } from "@/lib/bindings/RoomInfo"
import type { GameView } from "@/hooks/use-game-socket"
import { GameTable } from "@/screens/game-table"

interface RoomScreenProps {
  name: string
  room: RoomInfo
  game: GameView | null
  send: (msg: ClientMessage) => void
  leave: () => void
}

export function RoomScreen({ name, room, game, send, leave }: RoomScreenProps) {
  const inGame = game !== null && game.phase !== "lobby"

  return (
    <div className="mx-auto flex min-h-svh max-w-4xl flex-col gap-4 p-6">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-medium">{room.name}</h1>
        <Button size="sm" variant="outline" onClick={leave}>
          Leave
        </Button>
      </div>

      {inGame ? (
        <GameTable name={name} room={room} game={game} send={send} />
      ) : (
        <>
          <ul className="flex flex-col gap-1 text-sm">
            {room.players.map((p) => (
              <li key={p}>
                {p}
                {p === name && <span className="text-muted-foreground"> (you)</span>}
              </li>
            ))}
          </ul>
          <Button
            onClick={() => send({ type: "start_game" })}
            disabled={room.players.length !== 4}
          >
            Start game ({room.players.length}/4)
          </Button>
        </>
      )}
    </div>
  )
}
