// API-Client für alle HTTP-Anfragen an das Backend.

/** Custom API error with optional error code from server */
class ApiError extends Error {
  code?: string
  constructor(message: string, code?: string) {
    super(message)
    this.code = code
  }
}

async function checkAuth(r: Response, path: string): Promise<void> {
  if (r.status === 401) {
    window.location.href = '/';
    throw new ApiError('Session expired');
  }
  if (!r.ok) {
    // Try to parse structured error from server
    try {
      const body = await r.clone().json()
      throw new ApiError(body.error || `${r.status} ${path}`, body.code)
    } catch (e) {
      if (e instanceof ApiError) throw e
      throw new ApiError(`${r.status} ${path}`)
    }
  }
}

const api = {
  async get<T>(path: string): Promise<T> {
    const r = await fetch(path);
    await checkAuth(r, path);
    return r.json();
  },
  async post<T>(path: string, body: unknown): Promise<T> {
    const r = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    await checkAuth(r, path);
    return r.json();
  },
  async put<T>(path: string, body: unknown): Promise<T> {
    const r = await fetch(path, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    await checkAuth(r, path);
    return r.json();
  },
  async del(path: string): Promise<void> {
    const r = await fetch(path, { method: 'DELETE' });
    await checkAuth(r, path);
  },
};

export { ApiError };
export default api;
