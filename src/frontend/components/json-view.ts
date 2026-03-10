// Interaktive JSON-Tree-Ansicht.

import { escapeHtml } from '../utils';

export function renderJsonTree(obj: unknown, container: HTMLElement, depth: number = 0): void {
  container.innerHTML = '';
  buildTreeNode(obj, container, depth, '');
}

function buildTreeNode(value: unknown, parent: HTMLElement, depth: number, key: string): void {
  if (value === null || value === undefined) {
    const line = document.createElement('div');
    line.className = 'json-line';
    line.style.paddingLeft = (depth * 16) + 'px';
    line.innerHTML = (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
      + `<span class="json-value json-null">null</span>`;
    parent.appendChild(line);
    return;
  }

  if (Array.isArray(value)) {
    const wrapper = document.createElement('div');
    wrapper.className = 'json-node';

    const toggle = document.createElement('div');
    toggle.className = 'json-line json-toggle';
    toggle.style.paddingLeft = (depth * 16) + 'px';
    const collapsed = depth > 0;
    toggle.innerHTML = `<span class="json-arrow${collapsed ? '' : ' json-arrow-open'}">\u25B6</span>`
      + (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
      + `<span class="json-bracket">[</span>`
      + `<span class="json-collapsed-hint">${collapsed ? value.length + ' items' : ''}</span>`;

    const children = document.createElement('div');
    children.className = 'json-children';
    if (collapsed) children.style.display = 'none';

    value.forEach((item: unknown, i: number) => {
      buildTreeNode(item, children, depth + 1, String(i));
    });

    const closeBracket = document.createElement('div');
    closeBracket.className = 'json-line';
    closeBracket.style.paddingLeft = (depth * 16) + 'px';
    closeBracket.innerHTML = '<span class="json-bracket">]</span>';
    if (collapsed) closeBracket.style.display = 'none';

    toggle.addEventListener('click', () => {
      const isHidden = children.style.display === 'none';
      children.style.display = isHidden ? '' : 'none';
      closeBracket.style.display = isHidden ? '' : 'none';
      toggle.querySelector('.json-arrow')!.classList.toggle('json-arrow-open', isHidden);
      toggle.querySelector('.json-collapsed-hint')!.textContent = isHidden ? '' : value.length + ' items';
    });

    wrapper.appendChild(toggle);
    wrapper.appendChild(children);
    wrapper.appendChild(closeBracket);
    parent.appendChild(wrapper);
    return;
  }

  if (typeof value === 'object') {
    const keys = Object.keys(value as Record<string, unknown>);
    const wrapper = document.createElement('div');
    wrapper.className = 'json-node';

    const toggle = document.createElement('div');
    toggle.className = 'json-line json-toggle';
    toggle.style.paddingLeft = (depth * 16) + 'px';
    const collapsed = depth > 0;
    toggle.innerHTML = `<span class="json-arrow${collapsed ? '' : ' json-arrow-open'}">\u25B6</span>`
      + (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
      + `<span class="json-bracket">{</span>`
      + `<span class="json-collapsed-hint">${collapsed ? keys.length + ' keys' : ''}</span>`;

    const children = document.createElement('div');
    children.className = 'json-children';
    if (collapsed) children.style.display = 'none';

    keys.forEach((k: string) => {
      buildTreeNode((value as Record<string, unknown>)[k], children, depth + 1, k);
    });

    const closeBracket = document.createElement('div');
    closeBracket.className = 'json-line';
    closeBracket.style.paddingLeft = (depth * 16) + 'px';
    closeBracket.innerHTML = '<span class="json-bracket">}</span>';
    if (collapsed) closeBracket.style.display = 'none';

    toggle.addEventListener('click', () => {
      const isHidden = children.style.display === 'none';
      children.style.display = isHidden ? '' : 'none';
      closeBracket.style.display = isHidden ? '' : 'none';
      toggle.querySelector('.json-arrow')!.classList.toggle('json-arrow-open', isHidden);
      toggle.querySelector('.json-collapsed-hint')!.textContent = isHidden ? '' : keys.length + ' keys';
    });

    wrapper.appendChild(toggle);
    wrapper.appendChild(children);
    wrapper.appendChild(closeBracket);
    parent.appendChild(wrapper);
    return;
  }

  // Primitive values
  const line = document.createElement('div');
  line.className = 'json-line';
  line.style.paddingLeft = (depth * 16) + 'px';
  let cls = 'json-value';
  if (typeof value === 'string') cls += ' json-string';
  else if (typeof value === 'number') cls += ' json-number';
  else if (typeof value === 'boolean') cls += ' json-bool';

  const displayVal = typeof value === 'string' ? `"${escapeHtml(value)}"` : String(value);
  line.innerHTML = (key ? `<span class="json-key">${escapeHtml(key)}</span>: ` : '')
    + `<span class="${cls}">${displayVal}</span>`;
  parent.appendChild(line);
}

export function toggleJsonView(): void {
  const tree = document.getElementById('proj-json-tree')!;
  const textarea = document.getElementById('proj-modal-json') as HTMLTextAreaElement;
  const btn = document.getElementById('proj-view-toggle')!;
  if (textarea.style.display === 'none') {
    textarea.style.display = '';
    tree.style.display = 'none';
    btn.textContent = 'Tree';
  } else {
    try {
      const data = JSON.parse(textarea.value);
      renderJsonTree(data, tree);
    } catch { /* keep old tree */ }
    textarea.style.display = 'none';
    tree.style.display = '';
    btn.textContent = 'Raw JSON';
  }
}
