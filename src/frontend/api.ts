// API-Client für alle HTTP-Anfragen an das Backend.

function checkAuth(r: Response, path: string): void {
  if (r.status === 401) {
    // Session abgelaufen → zur Login-Seite
    window.location.href = '/';
    throw new Error('Session abgelaufen');
  }
  if (!r.ok) throw new Error(`${r.status} ${path}`);
}

const api = {
  async get<T>(path: string): Promise<T> {
    const r = await fetch(path);
    checkAuth(r, path);
    return r.json();
  },
  async post<T>(path: string, body: unknown): Promise<T> {
    const r = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    checkAuth(r, path);
    return r.json();
  },
  async put<T>(path: string, body: unknown): Promise<T> {
    const r = await fetch(path, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    checkAuth(r, path);
    return r.json();
  },
  async del(path: string): Promise<void> {
    const r = await fetch(path, { method: 'DELETE' });
    checkAuth(r, path);
  },
};

export default api;
