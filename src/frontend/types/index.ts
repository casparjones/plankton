// Zentrale TypeScript-Interfaces für das Plankton-Frontend.
// Basierend auf den Rust-Datenmodellen in src/models/project.rs und src/models/auth.rs.

// === Projekt-Datenmodell ===

/** Vollständiges Kanban-Projekt als flaches Dokument. */
export interface ProjectDoc {
  _id: string
  _rev?: string
  title: string
  columns: Column[]
  users: User[]
  tasks: Task[]
  git?: GitConfig | null
}

/** Git-Repository-Konfiguration für ein Projekt. */
export interface GitConfig {
  repo_url: string
  branch: string
  path: string
  enabled: boolean
  last_push?: string | null
  last_error?: string | null
}

/** Eine Spalte im Kanban-Board. */
export interface Column {
  id: string
  title: string
  slug: string
  order: number
  color: string
  hidden: boolean
  locked: boolean
}

/** Ein Teammitglied, das Aufgaben zugewiesen bekommen kann. */
export interface User {
  id: string
  name: string
  avatar: string
  role: string
}

/** Eine einzelne Aufgabe (Karte) im Board. */
export interface Task {
  id: string
  title: string
  description: string
  column_id: string
  column_slug: string
  previous_row: string
  assignee_ids: string[]
  labels: string[]
  order: number
  points: number
  worker: string
  creator: string
  logs: string[]
  comments: string[]
  created_at: string
  updated_at: string
  task_type: string
  blocks: string[]
  blocked_by: string[]
  parent_id: string
  subtask_ids: string[]
}

// === Auth-Datenmodell ===

/** Ein registrierter Benutzer (öffentliche Felder, ohne password_hash). */
export interface AuthUser {
  id: string
  username: string
  display_name: string
  role: string
  created_at: string
  updated_at: string
  active: boolean
}

/** JWT-Claims aus dem Token (vom /auth/me Endpoint). */
export interface Claims {
  sub: string
  username: string
  display_name: string
  role: string
  exp: number
  must_change_password: boolean
}

/** Login-Anfrage. */
export interface LoginRequest {
  username: string
  password: string
}

/** Passwort-Änderungs-Anfrage. */
export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

/** Admin: Neuen Benutzer anlegen. */
export interface CreateAuthUserRequest {
  username: string
  display_name: string
  password: string
  role: string
}

/** Admin: Benutzer aktualisieren. */
export interface UpdateAuthUserRequest {
  display_name?: string
  role?: string
  active?: boolean
}

/** Admin: Passwort zurücksetzen. */
export interface ResetPasswordRequest {
  password: string
}

/** Ein Agent-Token für API-Zugriff ohne Login. */
export interface AgentToken {
  id: string
  name: string
  token: string
  role: string
  active: boolean
  created_at: string
}

/** Anfrage zum Erstellen eines Agent-Tokens. */
export interface CreateTokenRequest {
  name: string
  role: string
}

/** Anfrage zum Aktualisieren eines Agent-Tokens. */
export interface UpdateTokenRequest {
  name?: string
  role?: string
  active?: boolean
}

// === Frontend-spezifische Typen ===

/** Zentraler Anwendungs-State. */
export interface AppState {
  projects: ProjectDoc[]
  project: ProjectDoc | null
  kanban: unknown | null
  editingTask: Task | null
  isNewTask: boolean
  selectedTasks: Set<string>
  eventSource: EventSource | null
  currentUser: Claims | null
  isDragging: boolean
  detailTask: Task | null
}

/** API-Fehler-Response vom Backend. */
export interface ApiError {
  error: string
}

/** Task-Verschiebung (POST /api/projects/:id/tasks/:task_id/move). */
export interface MoveTaskRequest {
  column_id: string
  order: number
}
