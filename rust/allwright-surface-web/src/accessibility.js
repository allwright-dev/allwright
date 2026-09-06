// Evaluated inside an IIFE with Allwright-owned accessibility helpers in scope.
// Tree collection is independent of serialization; no snapshot-specific string grammar.
function collectAccessibilitySnapshot({ mode = "default" } = {}) {
  const { computeAccessibleName, computeAccessibleDescription, getRole,
    hiddenSubtree, visible: isVisible, cssText, children, normalize } = allwrightA11y;
  const parent = element => element.assignedSlot || element.parentElement || element.getRootNode().host;
  const style = element => element.ownerDocument.defaultView.getComputedStyle(element);
  const nodeFor = (role, name = '') => ({ role, name, states: {}, properties: {}, children: [] });
  const root = nodeFor('document', document.title);
  const ai = mode === 'ai';
  const excluded = element => ['HEAD', 'SCRIPT', 'STYLE', 'TEMPLATE', 'NOSCRIPT'].includes(element.tagName);
  const prune = element => excluded(element) || (!ai && hiddenSubtree(element));
  const rendered = element => {
    if (style(element).visibility !== 'visible') return false;
    if (element.checkVisibility && !element.checkVisibility()) return false;
    return [...element.getClientRects()].some(rect => rect.width > 0 && rect.height > 0);
  };
  // State belongs to the document, so references survive captures, not navigation.
  const registryKey = Symbol.for('allwright.accessibility.references.v1');
  let registry;
  const references = new Map();
  if (ai) {
    registry = document[registryKey];
    if (!registry) {
      const token = Array.from(crypto.getRandomValues(new Uint32Array(4)), n => n.toString(16)).join('');
      registry = { prefix: `aw-${token}-`, next: 0, elements: new WeakMap(), previous: new Map() };
      Object.defineProperty(document, registryKey, { value: registry });
    }
  }
  function reference(element, node) {
    if (!ai || !rendered(element) || style(element).pointerEvents === 'none') return;
    let entry = registry.elements.get(element);
    if (!entry || entry.role !== node.role || entry.name !== node.name) {
      entry = { id: `${registry.prefix}${++registry.next}`, role: node.role, name: node.name };
      registry.elements.set(element, entry);
    }
    node['aria-ref'] = entry.id;
    references.set(element, entry.id);
  }
  const visited = new Set();
  const ownedBy = new Map();
  const owns = new Map();
  // Resolve ownership before traversal so aria-owns also works when the owner
  // follows the owned node in DOM order. Ignore cycles and hidden owners.
  const elements = [];
  const pending = document.documentElement ? [document.documentElement] : [];
  while (pending.length) {
    const element = pending.pop();
    if (element.nodeType !== 1 || prune(element)) continue;
    elements.push(element);
    pending.push(...children(element).slice().reverse());
  }
  for (const element of elements) {
    const owned = [];
    for (const id of (element.getAttribute('aria-owns') || '').split(/\s+/).filter(Boolean)) {
      const target = element.getRootNode().getElementById?.(id);
      if (!target || target === element || ownedBy.has(target) || prune(target)) continue;
      let cyclic = false;
      for (let ancestor = element; ancestor; ancestor = ownedBy.get(ancestor) || parent(ancestor)) {
        if (ancestor === target) { cyclic = true; break; }
      }
      if (!cyclic) { ownedBy.set(target, element); owned.push(target); }
    }
    owns.set(element, owned);
  }
  function addStates(element, node) {
    const { states, properties } = node;
    for (const key of ['busy', 'expanded', 'selected', 'required', 'readonly', 'multiselectable', 'modal', 'atomic']) {
      const value = element.getAttribute(`aria-${key}`);
      if (value === 'true' || value === 'false') states[key] = value === 'true';
    }
    for (const key of ['checked', 'pressed']) {
      const value = element.getAttribute(`aria-${key}`);
      const applicable = key === 'pressed' ? node.role === 'button' : ['checkbox', 'menuitemcheckbox', 'menuitemradio', 'radio', 'switch', 'option', 'treeitem'].includes(node.role);
      if (applicable && ['true', 'false', 'mixed'].includes(value)) {
        states[key] = value === 'mixed' && !['radio', 'menuitemradio', 'switch'].includes(node.role) ? 'mixed' : value === 'true';
      }
    }
    if (element.tagName === 'INPUT' && ['checkbox', 'radio'].includes(element.type)) {
      states.checked = element.type === 'checkbox' && element.indeterminate ? 'mixed' : element.checked;
    }
    if (element.tagName === 'OPTION') states.selected = element.selected;
    if (element.matches(':disabled')) states.disabled = true;
    else {
      for (let ancestor = element; ancestor; ancestor = parent(ancestor)) {
        const value = ancestor.getAttribute('aria-disabled');
        if (value === 'true' || value === 'false') { states.disabled = value === 'true'; break; }
      }
    }
    if (element.required) states.required = true;
    if (element.readOnly) states.readonly = true;
    if (element.tagName === 'DETAILS') states.expanded = element.open;
    if (element.tagName === 'SUMMARY' && element.parentElement?.tagName === 'DETAILS') states.expanded = element.parentElement.open;
    if (element === element.getRootNode().activeElement) states.focused = true;
    for (const key of ['invalid', 'current', 'haspopup']) {
      const value = element.getAttribute(`aria-${key}`);
      if (value !== null) states[key] = value === 'true' ? true : value === 'false' ? false : value;
    }
    for (const key of ['level', 'valuemin', 'valuemax', 'valuenow', 'posinset', 'setsize', 'rowcount', 'colcount', 'rowindex', 'colindex']) {
      const value = element.getAttribute(`aria-${key}`);
      if (value !== null && value.trim() && Number.isFinite(Number(value))) states[key] = Number(value);
    }
    if (node.role === 'heading' && states.level === undefined && /^H[1-6]$/.test(element.tagName)) states.level = Number(element.tagName[1]);
    for (const key of ['valuetext', 'roledescription', 'keyshortcuts', 'live', 'relevant', 'autocomplete', 'orientation']) {
      const value = element.getAttribute(`aria-${key}`);
      if (value !== null) properties[key] = value;
    }
    if (['INPUT', 'TEXTAREA', 'SELECT'].includes(element.tagName) &&
        !['password', 'file', 'checkbox', 'radio'].includes(element.type)) properties.value = element.value;
    if (['slider', 'spinbutton', 'progressbar', 'meter'].includes(node.role)) {
      for (const [attribute, key] of [['min', 'valuemin'], ['max', 'valuemax'], ['value', 'valuenow']]) {
        if (states[key] === undefined && typeof element[attribute] === 'number') states[key] = element[attribute];
        else if (states[key] === undefined && element.getAttribute(attribute)?.trim() && Number.isFinite(Number(element.getAttribute(attribute)))) states[key] = Number(element.getAttribute(attribute));
      }
    }
    if (element.hasAttribute('placeholder')) properties.placeholder = element.getAttribute('placeholder');
    if (node.role === 'link' && element.hasAttribute('href')) properties.url = element.getAttribute('href');
    if (node.role === 'iframe') properties.url = element.getAttribute('src') || 'about:blank';
  }
  function appendText(destination, value) {
    // Keep whitespace until the containing node is complete, preserving inline text.
    const last = destination.children.at(-1);
    if (last?.role === 'text') last.name += value;
    else destination.children.push(nodeFor('text', value));
  }
  function visit(node, destination, owner = null) {
    if (visited.has(node) || (ownedBy.has(node) && ownedBy.get(node) !== owner)) return;
    visited.add(node);
    if (node.nodeType === 3) { appendText(destination, node.nodeValue || ''); return; }
    if (node.nodeType !== 1 || prune(node)) return;
    const accessible = !hiddenSubtree(node) && isVisible(node);
    const visible = ai ? accessible || rendered(node) : mode === 'autoexpect' ? accessible && rendered(node) : accessible;
    const name = visible ? normalize(computeAccessibleName(node)) : '';
    const role = getRole(node) || (ai ? "generic" : null);
    const semantic = visible && role && !['none', 'presentation'].includes(role);
    const current = node === document.documentElement ? root : semantic ? nodeFor(role, name) : destination;
    if (current !== destination) destination.children.push(current);
    if (current !== destination || node === document.documentElement) {
      reference(node, current);
      addStates(node, current);
      const description = normalize(computeAccessibleDescription(node));
      if (description) current.properties.description = description;
    }
    const block = style(node).display !== 'inline' || node.tagName === 'BR';
    if (block) appendText(current, ' ');
    if (visible) appendText(current, cssText(node, '::before'));
    if (!['TEXTAREA', 'INPUT', 'IFRAME', 'FRAME'].includes(node.tagName)) {
      for (const child of children(node)) {
        if (child.nodeType !== 3 || visible) visit(child, current);
      }
    }
    for (const child of owns.get(node) || []) visit(child, current, node);
    if (visible) appendText(current, cssText(node, '::after'));
    if (block) appendText(current, ' ');
  }
  // Use a fragment destination to avoid putting the document root inside itself.
  if (document.documentElement) visit(document.documentElement, nodeFor('fragment'));
  const clean = [root];
  while (clean.length) {
    const node = clean.pop();
    node.children = node.children.filter(child => child.role !== 'text' || (child.name = normalize(child.name)));
    if (node.children.length === 1 && node.children[0].role === 'text' && node.children[0].name === node.name) node.children = [];
    clean.push(...node.children);
  }
  if (ai) {
    // Commit after traversal so attribute-sensitive styles cannot affect this capture.
    for (const [element, id] of registry.previous) {
      if (!references.has(element) && element.getAttribute('aria-ref') === id) element.removeAttribute('aria-ref');
    }
    for (const [element, id] of references) {
      if (element.getAttribute('aria-ref') !== id) element.setAttribute('aria-ref', id);
    }
    registry.previous = references;
  }
  return { url: document.URL, title: document.title, root };
}
