import type { Card } from "@/lib/bindings/Card"
import type { Rank } from "@/lib/bindings/Rank"
import type { Suit } from "@/lib/bindings/Suit"

export const ALL_SUITS: Suit[] = ["clubs", "diamonds", "hearts", "spades"]

export const SUIT_SYMBOL: Record<Suit, string> = {
  clubs: "♣",
  diamonds: "♦",
  hearts: "♥",
  spades: "♠",
}

export const RANK_LABEL: Record<Rank, string> = {
  nine: "9",
  ten: "10",
  jack: "J",
  queen: "Q",
  king: "K",
  ace: "A",
}

export const isRed = (suit: Suit) => suit === "diamonds" || suit === "hearts"

export const effectiveSuit = (card: Card, trump: Suit): Suit =>
  card.rank === "jack" && isRed(card.suit) === isRed(trump) ? trump : card.suit
