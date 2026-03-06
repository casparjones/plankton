const { createApp, ref, computed, onMounted, nextTick } = Vue;
const { createPinia, defineStore } = Pinia;

const useBoardStore = defineStore('board', {
  state: () => ({
    projects: [],
    activeProject: null,
    drawer: false,
  }),
  actions: {
    async fetchProjects() {
      const r = await fetch('/api/projects');
      this.projects = await r.json();
    },
    async openProject(id) {
      const r = await fetch(`/api/projects/${id}`);
      this.activeProject = await r.json();
    },
    async createProject(title) {
      const payload = {
        id: '',
        title,
        columns: [
          { id: crypto.randomUUID(), title: 'Todo', order: 0, color: '#90CAF9' },
          { id: crypto.randomUUID(), title: 'In Progress', order: 1, color: '#FFCC80' },
          { id: crypto.randomUUID(), title: 'Done', order: 2, color: '#A5D6A7' },
        ],
        users: [],
        tasks: [],
      };
      const r = await fetch('/api/projects', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
      this.activeProject = await r.json();
      await this.fetchProjects();
    },
    async saveTask(task) {
      const r = await fetch(`/api/projects/${this.activeProject.id}/tasks/${task.id}`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(task)
      });
      this.activeProject = await r.json();
      await this.fetchProjects();
    },
    async createTask(columnId) {
      const task = {
        id: '', title: 'New Task', description: '', column_id: columnId, assignee_ids: [], labels: [], order: 0, created_at: '', updated_at: ''
      };
      const r = await fetch(`/api/projects/${this.activeProject.id}/tasks`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(task)
      });
      this.activeProject = await r.json();
      await this.fetchProjects();
    },
    async moveTask(taskId, columnId, order) {
      const r = await fetch(`/api/projects/${this.activeProject.id}/tasks/${taskId}/move`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ column_id: columnId, order })
      });
      this.activeProject = await r.json();
      await this.fetchProjects();
    },
  }
});

const App = {
  setup() {
    const store = useBoardStore();
    const newProjectName = ref('');
    const selectedTask = ref(null);

    const sortedColumns = computed(() => (store.activeProject?.columns || []).slice().sort((a, b) => a.order - b.order));
    const tasksByColumn = (columnId) => (store.activeProject?.tasks || []).filter(t => t.column_id === columnId).sort((a, b) => a.order - b.order);

    const initDnD = async () => {
      await nextTick();
      sortedColumns.value.forEach((column) => {
        const el = document.getElementById(`col-${column.id}`);
        if (!el || el.dataset.init) return;
        el.dataset.init = '1';
        new Sortable(el, {
          group: 'tasks',
          animation: 150,
          onEnd: async (evt) => {
            const taskId = evt.item.dataset.taskId;
            const columnId = evt.to.dataset.columnId;
            await store.moveTask(taskId, columnId, evt.newIndex || 0);
          }
        });
      });
    };

    onMounted(async () => {
      await store.fetchProjects();
      if (store.projects[0]) await store.openProject(store.projects[0].id);
      await initDnD();
    });

    return { store, newProjectName, sortedColumns, tasksByColumn, selectedTask, initDnD };
  },
  watch: {
    'store.activeProject': {
      handler() { this.initDnD(); },
      deep: true,
    }
  },
  template: `
  <v-app theme="dark">
    <v-app-bar color="surface-variant" density="comfortable">
      <v-app-bar-title>Plankton</v-app-bar-title>
      <v-spacer></v-spacer>
      <v-text-field v-model="newProjectName" density="compact" hide-details label="New project" style="max-width:220px"></v-text-field>
      <v-btn class="ml-2" color="primary" @click="store.createProject(newProjectName || 'Untitled')">Create</v-btn>
    </v-app-bar>

    <v-container fluid class="py-4">
      <div v-if="!store.activeProject">
        <v-card><v-card-text>Create your first project.</v-card-text></v-card>
      </div>
      <div v-else>
        <h2 class="text-h5 mb-3">{{ store.activeProject.title }}</h2>
        <v-row>
          <v-col cols="12">
            <div class="board-scroll">
              <div v-for="column in sortedColumns" :key="column.id" class="column">
                <v-card>
                  <v-card-title :style="{ borderTop: '4px solid ' + column.color }">{{ column.title }}</v-card-title>
                  <v-card-text>
                    <v-btn size="small" variant="outlined" @click="store.createTask(column.id)">+ Task</v-btn>
                    <div :id="'col-' + column.id" :data-column-id="column.id" class="mt-3">
                      <v-card class="task-card" v-for="task in tasksByColumn(column.id)" :key="task.id" :data-task-id="task.id" @click="selectedTask = task">
                        <v-card-title class="text-subtitle-1">{{ task.title }}</v-card-title>
                        <v-card-subtitle>{{ task.labels.join(', ') }}</v-card-subtitle>
                      </v-card>
                    </div>
                  </v-card-text>
                </v-card>
              </div>
            </div>
          </v-col>
        </v-row>
      </div>
    </v-container>

    <v-navigation-drawer location="right" temporary v-model="selectedTask">
      <v-container v-if="selectedTask">
        <v-text-field label="Title" v-model="selectedTask.title" @change="store.saveTask(selectedTask)"></v-text-field>
        <v-textarea label="Description (Markdown)" v-model="selectedTask.description" @change="store.saveTask(selectedTask)"></v-textarea>
        <v-text-field label="Labels (comma separated)" :model-value="selectedTask.labels.join(',')" @change="(e) => { selectedTask.labels = e.target.value.split(',').map(x => x.trim()).filter(Boolean); store.saveTask(selectedTask);} "></v-text-field>
      </v-container>
    </v-navigation-drawer>
  </v-app>
  `
};

const vuetify = Vuetify.createVuetify({ theme: { defaultTheme: 'dark' } });
createApp(App).use(createPinia()).use(vuetify).mount('#app');
