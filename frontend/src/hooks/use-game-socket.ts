import { useRef, useState } from "react"
import { toast } from "sonner"

import { wsUrl } from "@/lib/api"
import type { Card } from "@/lib/bindings/Card"
import type { ClientMessage } from "@/lib/bindings/ClientMessage"
import type { Game } from "@/lib/bindings/Game"
import type { RoomInfo } from "@/lib/bindings/RoomInfo"
import type { ServerMessage } from "@/lib/bindings/ServerMessage"

export interface ChatLine {
  from: string
  text: string
}

/** Game state plus this player's private view: their cards, card counts for all seats. */
export type GameView = Game & { hand: Card[]; handCounts: number[] }

export function useGameSocket(name: string) {
  const [room, setRoom] = useState<RoomInfo | null>(null)
  const [game, setGame] = useState<GameView | null>(null)
  const [chat, setChat] = useState<ChatLine[]>([])
  const ws = useRef<WebSocket | null>(null)
  const token = useRef<string | undefined>(undefined)

  const join = (roomId: string) => {
    const sock = new WebSocket(wsUrl(roomId, name, token.current))
    sock.onmessage = (e) => {
      const msg: ServerMessage = JSON.parse(e.data)
      switch (msg.type) {
        case "joined":
          setRoom(msg.room)
          token.current = msg.token
          break
        case "player_joined":
          setRoom(
            (r) =>
              r && !r.players.includes(msg.name)
                ? { ...r, players: [...r.players, msg.name] }
                : r,
          )
          break
        case "player_left":
          setRoom((r) => r && { ...r, players: r.players.filter((p) => p !== msg.name) })
          break
        case "chat":
          setChat((c) => [...c, { from: msg.from, text: msg.text }])
          break
        case "game_state":
          setGame({ ...msg.game, hand: msg.hand, handCounts: msg.hand_counts })
          break
        case "error":
          toast.error(msg.message)
          break
      }
    }
    sock.onclose = () => {
      setRoom(null)
      setGame(null)
      setChat([])
      ws.current = null
    }
    ws.current = sock
  }

  const leave = () => ws.current?.close()
  const send = (msg: ClientMessage) => ws.current?.send(JSON.stringify(msg))

  return { room, game, chat, join, leave, send }
}
