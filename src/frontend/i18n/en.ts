export default {
  // ─── Common ───
  save: 'Save',
  cancel: 'Cancel',
  delete: 'Delete',
  edit: 'Edit',
  create: 'Create',
  close: 'Close',
  send: 'Send',
  yes: 'Yes',
  no: 'No',
  error: 'Error',
  copied: 'Copied',
  download: 'Download',
  copyToClipboard: 'Copy to clipboard',
  loading: 'Loading...',
  none: '–',

  // ─── Login ───
  login: {
    username: 'Username',
    password: 'Password',
    submit: 'Sign in',
    failed: 'Login failed',
  },

  // ─── Sidebar ───
  sidebar: {
    projectPlaceholder: 'Project name...',
    changeTheme: 'Toggle theme',
    changePassword: 'Change password',
    admin: 'Admin',
    logout: 'Sign out',
    noUsers: 'No users',
  },

  // ─── Board ───
  board: {
    search: 'Search',
    searchPlaceholder: 'Search title & description... (Esc to close)',
    allLabels: 'All labels',
    allWorkers: 'All workers',
    resetFilters: 'Reset filters',
    closeSearch: 'Close search',
    noTasks: 'No tasks',
    addTask: 'Add task',
    manageColumn: 'Manage column',
    projectMenu: 'Project menu',
    importIssues: 'Import',
  },

  // ─── Bulk actions ───
  bulk: {
    selected: '{count} task(s) selected',
    deleteSelected: 'Delete selected',
    deselectAll: 'Deselect all',
    deleteConfirm: 'Delete {count} task(s)?',
    deleteError: 'Error deleting:',
  },

  // ─── Task Modal ───
  taskModal: {
    newTask: 'New Task',
    editTask: 'Edit Task',
    title: 'Title',
    description: 'Description',
    labels: 'Labels',
    labelsHint: 'comma-separated',
    comments: 'Comments',
    noComments: 'No comments',
    commentPlaceholder: 'Write a comment...',
    type: 'Type',
    parentEpic: 'Parent Epic',
    points: 'Points',
    worker: 'Worker',
    created: 'Created',
    modified: 'Modified',
    previousColumn: 'Previous column',
    blockedBy: 'Blocked by',
    searchTask: 'Search task...',
    logs: 'Logs',
    noLogs: 'No logs',
    deleteConfirm: 'Delete task "{title}"?',
  },

  // ─── Task Detail ───
  taskDetail: {
    details: 'Details',
    subtasks: 'Subtasks',
    done: 'done',
    relatedTickets: 'Related Tickets',
    mcpLink: 'MCP Link',
    mcpLinkCopied: 'Copied',
    mcpLinkTitle: 'Copy MCP link for Claude Code',
  },

  // ─── Task types ───
  taskType: {
    task: 'Task',
    epic: 'Epic',
    job: 'Job',
  },

  // ─── Column Modal ───
  column: {
    column: 'Column',
    editColumn: 'Edit column',
    newColumn: 'New column',
    title: 'Title',
    titlePlaceholder: 'Column name...',
    color: 'Color',
    moveLeft: 'Move left',
    moveRight: 'Move right',
    deleteColumn: 'Delete column',
    deleteConfirm: 'Really delete column "{title}" and {count} task(s)?',
    deleteConfirmEmpty: 'Really delete column "{title}"?',
    locked: 'locked',
  },

  // ─── Project Modal ───
  project: {
    project: 'Project',
    projectName: 'Project name',
    projectPlaceholder: 'Project name...',
    editProject: 'Edit project',
    importAsNew: 'Import as new project',
    rawJson: 'Raw JSON',
  },

  // ─── Git ───
  git: {
    settings: 'Git Settings',
    repoUrl: 'Repository URL',
    branch: 'Branch',
    pathInRepo: 'Path in repository',
    autoSync: 'Auto-sync enabled',
    syncNow: 'Sync now',
    syncing: 'Syncing...',
    syncSuccess: 'Success!',
    syncFailed: 'Failed',
    syncError: 'Error!',
    disabled: 'Git sync disabled',
    error: 'Git error: {error}',
    lastPush: 'Last git push: {date}',
    configured: 'Git configured, no push yet',
    notConfigured: 'Not configured yet',
    lastPushStatus: 'Last push: {date}',
    errorStatus: 'Error: {error}',
    noSync: 'No sync performed yet',
    repoRequired: 'Repository URL is required',
  },

  // ─── Prompt / AI Agents ───
  prompt: {
    aiAgents: 'AI Agents',
    simple: 'Simple',
    generateFiles: 'Generate files',
    agentTokens: 'Agent Tokens',
    tokensHint: 'Tokens can be managed under <strong>Admin ({icon}) → Tokens</strong>.',
    loadingTokens: 'Loading tokens...',
    installCli: 'Plankton CLI',
    installation: 'Installation',
    installHint: 'Install the Plankton CLI with a single command:',
    loginTitle: 'Login',
    loginHint: 'Add server and log in (like <code>{code}</code>):',
    claudeCodeSkill: 'Claude Code Skill',
    skillHint: 'Install skill (incl. login + secrets setup):',
    help: 'Help',
    connectorTitle: 'Plankton as Connector in claude.ai',
    connectorDesc: 'Plankton can be integrated as a custom MCP connector in claude.ai, allowing Claude to access the Kanban board directly.',
    connectorStep1: '1. Add Connector in claude.ai',
    connectorStep1Desc: 'In claude.ai under <strong>Settings → Connectors → Add custom connector</strong>:',
    connectorServerUrl: 'Server URL',
    connectorOAuthHint: 'Claude automatically detects the OAuth endpoints via <code>{endpoint}</code> and registers via Dynamic Client Registration.',
    connectorStep2: '2. Authorize',
    connectorStep2Desc: 'On first access, claude.ai opens a login window. Sign in with your Plankton account – done.',
    connectorOAuthFlow: 'OAuth 2.0 Authorization Code Flow with PKCE and Refresh Token Rotation. Callback URL:',
    planktonUrl: 'Plankton URL',
    claudeCodeSetup: 'Claude Code Setup',
    installSkillHint: 'Install the Plankton Skill for Claude Code with the CLI:',
    cliAutoLogin: 'The CLI handles login automatically and sets up the secrets.',
  },

  // ─── Admin ───
  admin: {
    administration: 'Administration',
    users: 'Users',
    tokens: 'Tokens',
    username: 'Username',
    displayName: 'Display name',
    password: 'Password',
    role: 'Role',
    newUser: 'New user',
    noUsers: 'No users',
    noTokens: 'No tokens',
    deactivate: 'Deactivate',
    activate: 'Activate',
    pwReset: 'PW Reset',
    unchanged: '(unchanged)',
    tokenCreated: 'Token created – copy now, it won\'t be shown again!',
    newPasswordPrompt: 'New password:',
    passwordReset: 'Password has been reset',
    tokenName: 'Token name...',
  },

  // ─── Password Modal ───
  passwordModal: {
    changePassword: 'Change Password',
    oldPassword: 'Old password',
    newPassword: 'New password',
    confirmPassword: 'Confirm new password',
    mismatch: 'Passwords do not match',
    tooShort: 'Password must be at least 4 characters',
    changeFailed: 'Password change failed',
  },

  // ─── Import ───
  import: {
    importIssues: 'Import Issues',
    jsonLabel: 'JSON (Array of Tasks)',
    validate: 'Validate',
    startImport: 'Start import',
    importing: 'Importing...',
    invalidJson: 'Invalid JSON',
    validSummary: '<strong>{valid}</strong> valid, <strong>{errors}</strong> errors',
    tableTitle: 'Title',
    tableColumn: 'Column',
    tablePoints: 'Points',
    tableNotes: 'Notes',
    importError: 'Error: {error}',
    // Mobile import page
    mobileImport: 'Mobile Import',
    backToBoard: '← Board',
    projectLabel: 'Project',
    newProject: '+ New project',
    tasksJson: 'Tasks (JSON)',
    paste: 'Paste',
    check: 'Validate',
    importBtn: 'Import',
    supervisorPrompt: 'Supervisor Prompt',
    copyPrompt: 'Copy prompt',
  },

  // ─── Drag & Drop ───
  drag: {
    moveFailed: 'Move failed',
    blockedBy: 'Blocked by: {blockers}',
    BLOCKED_BY: 'Cannot move to Done — blocked by: {details}',
  },

  // ─── Auth ───
  auth: {
    loginFailed: 'Login failed',
  },
} as const
