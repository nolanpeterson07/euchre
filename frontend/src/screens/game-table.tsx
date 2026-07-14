import { useEffect, useState } from "react"

import { CardBack, PlayingCard } from "@/components/playing-card"
import { Button } from "@/components/ui/button"
import type { Card } from "@/lib/bindings/Card"
import type { ClientMessage } from "@/lib/bindings/ClientMessage"
import type { RoomInfo } from "@/lib/bindings/RoomInfo"
import type { GameView } from "@/hooks/use-game-socket"
import { Checkbox } from "@/components/ui/checkbox"
import { ALL_SUITS, effectiveSuit, isRed, SUIT_SYMBOL } from "@/lib/cards"

interface GameTableProps {
  name: string
  room: RoomInfo
  game: GameView
  send: (msg: ClientMessage) => void
}

const TRICK_PAUSE_MS = 3000

export function GameTable({ name, room, game, send }: GameTableProps) {
  const [alone, setAlone] = useState(false)

  const tricksPlayed = game.teams[0].tricks_won + game.teams[1].tricks_won
  const [seenTricks, setSeenTricks] = useState(0)
  const showLastTrick = tricksPlayed > 0 && seenTricks < tricksPlayed
  useEffect(() => {
    const t = setTimeout(() => setSeenTricks(tricksPlayed), TRICK_PAUSE_MS)
    return () => clearTimeout(t)
  }, [tricksPlayed])
  const mySeat = room.players.indexOf(name)
  const myTurn = game.turn === mySeat
  const myHand = game.hand
  const left = (mySeat + 1) % 4
  const partner = (mySeat + 2) % 4
  const right = (mySeat + 3) % 4
  const myTeam = mySeat % 2

  const seatLabel = (seat: number) => (
    <span className="text-xs">
      {room.players[seat]}
      {seat === game.dealer && " (dealer)"}
      {seat === game.turn && " ←"}
    </span>
  )

  const isPlayable = (card: Card) => {
    if (game.phase !== "playing" || !game.trump) return true
    const lead = game.trick[0]
    if (!lead) return true
    const led = effectiveSuit(lead.card, game.trump)
    if (effectiveSuit(card, game.trump) === led) return true
    return !myHand.some((c) => effectiveSuit(c, game.trump!) === led)
  }

  const makerTeam = (game.maker?.player ?? 0) % 2
  const makerTricks = game.teams[makerTeam].tricks_won
  const handResult =
    makerTricks < 3
      ? `${makerTeam === myTeam ? "We" : "They"} got euchred!`
      : `${makerTeam === myTeam ? "We" : "They"} made it with ${makerTricks} tricks`

  const cardAction: ((card: Card) => void) | undefined = !myTurn
    ? undefined
    : game.phase === "playing"
      ? (card) => send({ type: "play_card", card })
      : game.phase === "awaiting_discard"
        ? (card) => send({ type: "discard", card })
        : undefined

  return (
    <div className="flex flex-col gap-4">
      {/* scores + trump */}
      <div className="flex items-center justify-between text-sm">
        <span>
          Us {game.teams[myTeam].score} ({game.teams[myTeam].tricks_won} tricks) ·
          Them {game.teams[1 - myTeam].score} ({game.teams[1 - myTeam].tricks_won})
        </span>
        {game.trump && <span>Trump: {SUIT_SYMBOL[game.trump]}</span>}
      </div>

      {/* diamond: partner top, opponents left/right vertically centered, trick in the middle */}
      <div className="grid min-h-[26rem] grid-cols-[6rem_1fr_6rem] grid-rows-[auto_1fr] items-center gap-6">
        <div />
        <div className="flex flex-col items-center gap-2">
          {seatLabel(partner)}
          <div className="deal-stagger flex -space-x-9">
            {Array.from({ length: game.handCounts[partner] }, (_, i) => (
              <CardBack key={i} className="h-16 w-11" />
            ))}
          </div>
        </div>
        <div />

        <div className="flex flex-col items-center gap-2 self-center">
          {seatLabel(left)}
          <div className="deal-stagger flex flex-col -space-y-9">
            {Array.from({ length: game.handCounts[left] }, (_, i) => (
              <CardBack key={i} className="h-16 w-11" />
            ))}
          </div>
        </div>

        <div className="flex flex-col items-center justify-center gap-2 self-stretch">
          <div className="flex items-center justify-center gap-4">
            {(showLastTrick ? game.last_trick : game.trick).map((p) => (
              <div key={p.player} className="play-in flex flex-col items-center gap-1">
                <PlayingCard
                  card={p.card}
                  className={
                    showLastTrick && p.player === game.trick_winner
                      ? "ring-2 ring-ring"
                      : undefined
                  }
                />
                <span className="text-xs text-muted-foreground">
                  {room.players[p.player]}
                </span>
              </div>
            ))}
            {game.upcard && (game.phase === "bidding1" || game.phase === "bidding2") && (
              <div className="play-in flex flex-col items-center gap-1">
                <PlayingCard card={game.upcard} />
                <span className="text-xs text-muted-foreground">upcard</span>
              </div>
            )}
          </div>
          {showLastTrick && game.trick_winner != null && (
            <p className="text-xs font-medium">
              {game.trick_winner === mySeat
                ? "You took the trick"
                : `${room.players[game.trick_winner]} took the trick`}
            </p>
          )}
        </div>

        <div className="flex flex-col items-center gap-2 self-center">
          {seatLabel(right)}
          <div className="deal-stagger flex flex-col -space-y-9">
            {Array.from({ length: game.handCounts[right] }, (_, i) => (
              <CardBack key={i} className="h-16 w-11" />
            ))}
          </div>
        </div>
      </div>

      {/* phase actions */}
      {game.phase === "game_over" ? (
        <p className="text-center text-sm font-medium">
          {game.teams[myTeam].score >= 10 ? "You win!" : "You lose."}
        </p>
      ) : game.phase === "hand_over" ? (
        <div className="flex flex-col items-center gap-2">
          <p className="text-sm font-medium">{handResult}</p>
          <Button size="sm" onClick={() => send({ type: "next_hand" })}>
            Next hand
          </Button>
        </div>
      ) : myTurn ? (
        <div className="flex flex-wrap items-center justify-center gap-2">
          {(game.phase === "bidding1" || game.phase === "bidding2") && (
            <>
              <label className="flex items-center gap-1.5 text-xs">
                <Checkbox
                  checked={alone}
                  onCheckedChange={(checked) => setAlone(checked === true)}
                />
                go alone
              </label>
              {game.phase === "bidding1" && (
                <Button size="sm" onClick={() => send({ type: "order_up", alone })}>
                  Order up
                </Button>
              )}
              {game.phase === "bidding2" &&
                ALL_SUITS.filter((s) => s !== game.upcard?.suit).map((suit) => (
                  <Button
                    key={suit}
                    size="sm"
                    variant="outline"
                    className={isRed(suit) ? "text-red-600" : undefined}
                    onClick={() => send({ type: "call_trump", suit, alone })}
                  >
                    {SUIT_SYMBOL[suit]}
                  </Button>
                ))}
              <Button size="sm" variant="outline" onClick={() => send({ type: "pass" })}>
                Pass
              </Button>
            </>
          )}
          {game.phase === "awaiting_discard" && (
            <span className="text-sm">Pick a card to discard.</span>
          )}
          {game.phase === "playing" && (
            <span className="text-sm">Your turn — play a card.</span>
          )}
        </div>
      ) : (
        <p className="text-center text-sm text-muted-foreground">
          Waiting on {room.players[game.turn]}…
        </p>
      )}

      {/* my hand */}
      <div className="deal-stagger flex justify-center gap-2">
        {myHand.map((card) => (
          <PlayingCard
            key={`${card.rank}-${card.suit}`}
            card={card}
            disabled={!isPlayable(card)}
            onClick={cardAction && (() => cardAction(card))}
          />
        ))}
      </div>
    </div>
  )
}
