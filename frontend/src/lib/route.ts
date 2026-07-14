const ROOM_PATH = /^\/room\/([^/]+)\/?$/

export const roomIdFromUrl = (): string | null =>
  window.location.pathname.match(ROOM_PATH)?.[1] ?? null

export const isKnownPath = (): boolean =>
  window.location.pathname === "/" || ROOM_PATH.test(window.location.pathname)

export const setRoomUrl = (id: string | null) => {
  const path = id ? `/room/${id}` : "/"
  if (window.location.pathname !== path) {
    window.history.pushState(null, "", path)
  }
}
