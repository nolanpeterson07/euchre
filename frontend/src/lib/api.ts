import type { RoomInfo } from "@/lib/bindings/RoomInfo"

export const listRooms = (): Promise<RoomInfo[]> =>
  fetch("/rooms").then((r) => r.json())

export const createRoom = (name: string): Promise<RoomInfo> =>
  fetch("/rooms", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name }),
  }).then((r) => r.json())

export const getRoom = (id: string): Promise<RoomInfo | null> =>
  fetch(`/rooms/${id}`).then((r) => (r.ok ? r.json() : null))

export const wsUrl = (roomId: string, player: string, token?: string) => {
  const proto = location.protocol === "https:" ? "wss" : "ws"
  const query = new URLSearchParams({ name: player, ...(token && { token }) })
  return `${proto}://${location.host}/ws/${roomId}?${query}`
}
