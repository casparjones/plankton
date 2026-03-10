// Typisierter API-Client für alle HTTP-Anfragen an das Backend.

/** Führt einen GET-Request aus und gibt die JSON-Response typisiert zurück. */
async function get<T>(path: string): Promise<T> {
  const r = await fetch(path)
  if (!r.ok) throw new Error(`GET ${path} → ${r.status}`)
  return r.json() as Promise<T>
}

/** Führt einen POST-Request mit JSON-Body aus. */
async function post<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!r.ok) throw new Error(`POST ${path} → ${r.status}`)
  return r.json() as Promise<T>
}

/** Führt einen PUT-Request mit JSON-Body aus. */
async function put<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(path, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!r.ok) throw new Error(`PUT ${path} → ${r.status}`)
  return r.json() as Promise<T>
}

/** Führt einen DELETE-Request aus (keine JSON-Response erwartet). */
async function del(path: string): Promise<void> {
  const r = await fetch(path, { method: 'DELETE' })
  if (!r.ok) throw new Error(`DELETE ${path} → ${r.status}`)
}

/** Composable für den API-Client. */
export function useApi() {
  return { get, post, put, del }
}

export default { get, post, put, del }
