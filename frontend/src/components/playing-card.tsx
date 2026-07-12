import type { Card } from "@/lib/bindings/Card"
import { isRed, RANK_LABEL, SUIT_SYMBOL } from "@/lib/cards"
import { cn } from "@/lib/utils"

interface PlayingCardProps {
  card: Card
  onClick?: () => void
  selected?: boolean
  disabled?: boolean
  className?: string
}

export function PlayingCard({
  card,
  onClick,
  selected,
  disabled,
  className,
}: PlayingCardProps) {
  const rank = RANK_LABEL[card.rank]
  const suit = SUIT_SYMBOL[card.suit]
  const clickable = !!onClick && !disabled

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={!clickable}
      aria-label={`${rank} of ${card.suit}`}
      className={cn(
        "relative flex h-24 w-16 shrink-0 flex-col justify-between rounded-lg border border-neutral-300 bg-white p-1 font-semibold shadow-sm select-none",
        isRed(card.suit) ? "text-red-600" : "text-neutral-900",
        clickable &&
          "cursor-pointer transition-transform hover:-translate-y-1.5 focus-visible:ring-2 focus-visible:ring-ring",
        selected && "-translate-y-1.5 ring-2 ring-ring",
        disabled && "opacity-60",
        className,
      )}
    >
      <span className="text-left text-xs leading-none">
        {rank}
        <br />
        {suit}
      </span>
      <span className="absolute inset-0 flex items-center justify-center text-3xl">
        {suit}
      </span>
      <span className="rotate-180 text-left text-xs leading-none">
        {rank}
        <br />
        {suit}
      </span>
    </button>
  )
}

export function CardBack({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "h-24 w-16 shrink-0 rounded-lg border border-neutral-300 bg-[repeating-linear-gradient(45deg,var(--color-blue-800),var(--color-blue-800)_4px,var(--color-blue-600)_4px,var(--color-blue-600)_8px)] shadow-sm",
        className,
      )}
    />
  )
}
