// Type-Deklarationen für Legacy-JS-Module.
// Erlaubt typsicheren Import von noch nicht migrierten .js-Dateien.

declare module '../utils.js' {
  export function escapeHtml(str: string): string
  export function columnName(colId: string): string
  export function formatDate(isoStr: string): string
}

declare module './utils.js' {
  export function escapeHtml(str: string): string
  export function columnName(colId: string): string
  export function formatDate(isoStr: string): string
}

declare module '../components/auth.js' {
  export function checkAuth(): Promise<any>
  export function doLogin(username: string, password: string): Promise<void>
  export function doLogout(callback: () => void): void
  export function doChangePassword(oldPassword: string, newPassword: string): Promise<void>
  export function updateUserSection(): void
}

declare module './components/auth.js' {
  export function checkAuth(): Promise<any>
  export function doLogin(username: string, password: string): Promise<void>
  export function doLogout(callback: () => void): void
  export function doChangePassword(oldPassword: string, newPassword: string): Promise<void>
  export function updateUserSection(): void
}

declare module '../components/board.js' {
  export function renderBoard(): void
}

declare module './components/board.js' {
  export function renderBoard(): void
}

declare module '../components/bulk-actions.js' {
  export function updateBulkBar(): void
  export function bulkDeleteSelected(): void
}

declare module './components/bulk-actions.js' {
  export function updateBulkBar(): void
  export function bulkDeleteSelected(): void
}

declare module '../components/column-modal.js' {
  export function openColumnMenu(anchorEl: any, colId: string): void
  export function closeColumnMenu(): void
  export function openColumnEditModal(colId: string): void
  export function openColumnAddModal(): void
  export function closeColumnModal(): void
  export function saveColumnModal(): void
  export function selectColor(color: string): void
  export function reorderColumnsFromDOM(): void
}

declare module './components/column-modal.js' {
  export function openColumnMenu(anchorEl: any, colId: string): void
  export function closeColumnMenu(): void
  export function closeColumnModal(): void
  export function saveColumnModal(): void
  export function selectColor(color: string): void
  export function reorderColumnsFromDOM(): void
}

declare module '../components/project-menu.js' {
  export function openProjectDropdown(): void
  export function closeProjectDropdown(): void
  export function openPromptModal(): void
  export function closePromptModal(): void
  export function openProjectMenu(): void
  export function closeProjectMenu(): void
  export function copyProjectJson(): void
  export function importProjectJson(): void
  export function saveProjectJson(): void
  export function saveProjectTitle(): void
}

declare module './components/project-menu.js' {
  export function openProjectDropdown(): void
  export function closeProjectMenu(): void
  export function copyProjectJson(): void
  export function importProjectJson(): void
  export function saveProjectJson(): void
  export function saveProjectTitle(): void
  export function closePromptModal(): void
}

declare module '../components/json-view.js' {
  export function renderJsonTree(obj: any, container: HTMLElement, depth?: number): void
  export function toggleJsonView(): void
}

declare module './components/json-view.js' {
  export function toggleJsonView(): void
}

declare module '../components/admin.js' {
  export const adminState: any
  export function openAdminModal(): void
  export function closeAdminModal(): void
  export function showAdminForm(user: any): void
  export function saveAdminForm(): void
  export function switchAdminTab(tab: string): void
  export function createToken(): void
  export function handleTokenAction(action: string, tid: string): void
  export function handleAdminUserAction(action: string, uid: string): void
}

declare module './components/admin.js' {
  export function openAdminModal(): void
  export function closeAdminModal(): void
  export function showAdminForm(user: any): void
  export function saveAdminForm(): void
  export function switchAdminTab(tab: string): void
  export function createToken(): void
  export function handleTokenAction(action: string, tid: string): void
  export function handleAdminUserAction(action: string, uid: string): void
}

declare module '../components/password-modal.js' {
  export function openPasswordModal(force: boolean): void
  export function closePasswordModal(): void
  export function savePassword(): void
}

declare module './components/password-modal.js' {
  export function openPasswordModal(force: boolean): void
  export function closePasswordModal(): void
  export function savePassword(): void
}

declare module '../components/import-modal.js' {
  export function openImportModal(): void
  export function closeImportModal(): void
  export function validateImport(): void
  export function executeImport(): void
}

declare module './components/import-modal.js' {
  export function openImportModal(): void
  export function closeImportModal(): void
  export function validateImport(): void
  export function executeImport(): void
}

declare module '../components/git-settings.js' {
  export function updateGitStatusIcon(): void
  export function openGitModal(): void
  export function closeGitModal(): void
  export function saveGitConfig(): void
  export function triggerGitSync(): void
}

declare module './components/git-settings.js' {
  export function updateGitStatusIcon(): void
  export function openGitModal(): void
  export function closeGitModal(): void
  export function saveGitConfig(): void
  export function triggerGitSync(): void
}

declare module '../components/theme.js' {
  export function applyTheme(theme: string): void
  export function toggleTheme(): void
  export function initTheme(): void
}

declare module './components/theme.js' {
  export function applyTheme(theme: string): void
  export function toggleTheme(): void
  export function initTheme(): void
}

// Globale Erweiterungen für Bridge-Funktionen
interface Window {
  __kanbanRefresh?: () => void
  __openNewTaskModal?: (columnId: string) => void
  __openTaskModal?: (task: any) => void
  __closeTaskModal?: () => void
  __openTaskDetail?: (task: any) => void
  __closeTaskDetail?: () => void
}
