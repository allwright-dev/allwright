// Allwright-owned locator evaluation. Executed within the target BiDi realm.
function allwrightQuery(segments, initialRoots = [document]) {
  const a = allwrightA11y;
  const unique = nodes => [...new Set(nodes)];
  const parent = e => e.parentElement || e.getRootNode().host;
  function descendants(root) {
    const result = [];
    function visit(node) {
      for (const child of node.children || []) {
        result.push(child);
        visit(child);
      }
      if (node.shadowRoot) visit(node.shadowRoot);
    }
    visit(root);
    return result;
  }
  const normalize = value => String(value ?? '').replace(/[\s\u200b\u00ad]+/g, ' ').trim();
  function matches(value, expected, exact = false) {
    if (expected === undefined) return true;
    if (expected && typeof expected === 'object' && typeof expected.regex === 'string') {
      return new RegExp(expected.regex, expected.flags || '').test(value);
    }
    if (typeof expected !== 'string') throw new Error('Text matcher must be a string or regular expression');
    const actual = normalize(value), wanted = normalize(expected);
    return exact ? actual === wanted : actual.toLowerCase().includes(wanted.toLowerCase());
  }
  function text(e) {
    if (['script', 'style', 'noscript'].includes(e.localName) || e.closest?.('head')) return '';
    if (e.localName === 'input' && ['button', 'submit'].includes(e.type)) return e.value;
    return [...e.childNodes].map(n => n.nodeType === 3 ? n.nodeValue : n.nodeType === 1 ? text(n) : '').join('') + (e.shadowRoot ? text(e.shadowRoot) : '');
  }
  function visible(e) {
    const style = getComputedStyle(e), box = e.getBoundingClientRect();
    return style.visibility !== 'hidden' && style.visibility !== 'collapse' && box.width > 0 && box.height > 0;
  }
  function state(e, key) {
    if (key === 'disabled') {
      if (e.matches(':disabled')) return true;
      for (let node = e; node; node = parent(node)) {
        const value = node.getAttribute('aria-disabled');
        if (value === 'true' || value === 'false') return value === 'true';
      }
      return false;
    }
    if (key === 'checked' && e.localName === 'input' && ['checkbox', 'radio'].includes(e.type)) return e.indeterminate ? 'mixed' : e.checked;
    if (key === 'selected' && e.localName === 'option') return e.selected;
    if (key === 'expanded' && e.localName === 'summary' && e.parentElement?.localName === 'details') return e.parentElement.open;
    if (key === 'level') return Number(e.getAttribute('aria-level') || (/^h[1-6]$/.test(e.localName) ? e.localName[1] : 0));
    const value = e.getAttribute(`aria-${key}`);
    return value === 'mixed' ? 'mixed' : value === 'true' ? true : value === 'false' ? false : undefined;
  }
  function parse(selector) {
    const result = [];
    let rest = selector.trim();
    while (rest) {
      const prefix = /^(css|xpath|aw)[=:]/i.exec(rest);
      if (!prefix) throw new Error('Invalid nested locator transport');
      rest = rest.slice(prefix[0].length);
      const quoted = /^"(?:[^"\\]|\\.)*"/.exec(rest);
      if (!quoted) throw new Error('Invalid nested locator payload');
      result.push({kind: prefix[1].toLowerCase(), value: JSON.parse(quoted[0])});
      rest = rest.slice(quoted[0].length).trim();
    }
    return result;
  }
  let roots = initialRoots;
  for (const segment of segments) {
    if (segment.kind === 'aw') {
      const spec = JSON.parse(segment.value);
      if (spec.chain) { roots = allwrightQuery(spec.chain, roots); continue; }
      if (spec.kind === 'exclude') {
        const excluded = new Set(allwrightQuery(parse(spec.selector), initialRoots));
        roots = roots.filter(element => !excluded.has(element));
        continue;
      }
      if (spec.kind === 'filter') {
        roots = roots.filter(e => {
          if (spec.visible !== undefined && visible(e) !== spec.visible) return false;
          if (spec.hasText !== undefined && !matches(text(e), spec.hasText)) return false;
          if (spec.hasNotText !== undefined && matches(text(e), spec.hasNotText)) return false;
          if (spec.has !== undefined && !allwrightQuery(parse(spec.has), [e]).length) return false;
          if (spec.hasNot !== undefined && allwrightQuery(parse(spec.hasNot), [e]).length) return false;
          return true;
        });
        continue;
      }
      if (spec.kind === 'nth') {
        const index = spec.index < 0 ? roots.length + spec.index : spec.index;
        roots = roots[index] ? [roots[index]] : [];
        continue;
      }
      const candidates = unique(roots.flatMap(descendants));
      roots = candidates.filter(e => {
        if (spec.kind === 'role') {
          if (a.getRole(e) !== spec.role || (!spec.includeHidden && !a.visible(e))) return false;
          if (!matches(a.computeAccessibleName(e, spec.includeHidden), spec.name, spec.exact)) return false;
          return ['checked', 'disabled', 'expanded', 'level', 'pressed', 'selected'].every(key => spec[key] === undefined || state(e, key) === spec[key]);
        }
        if (spec.kind === 'text') {
          if (['script', 'style', 'noscript'].includes(e.localName) || e.closest('head')) return false;
          return matches(text(e), spec.text, spec.exact) && !descendants(e).some(child => matches(text(child), spec.text, spec.exact));
        }
        if (spec.kind === 'label') {
          const refs = (e.getAttribute('aria-labelledby') || '').split(/\s+/).map(id => e.getRootNode().getElementById(id)).filter(Boolean);
          const labels = refs.length ? [refs.map(text).join(' ')] : e.hasAttribute('aria-label') ? [e.getAttribute('aria-label')] : [...(e.labels || [])].map(text);
          return labels.some(label => matches(label, spec.text, spec.exact));
        }
        const attr = {placeholder: 'placeholder', altText: 'alt', title: 'title', testId: 'data-testid'}[spec.kind];
        if (!attr) throw new Error(`Unknown semantic selector: ${spec.kind}`);
        if (spec.kind === 'altText' && !['img', 'input', 'area'].includes(e.localName)) return false;
        if (spec.kind === 'testId' && typeof spec.text === 'string') return e.getAttribute(attr) === spec.text;
        return e.hasAttribute(attr) && matches(e.getAttribute(attr), spec.text, spec.kind === 'testId' || spec.exact);
      });
      continue;
    }
    roots = unique(roots.flatMap(root => {
      if (segment.kind === 'css') {
        const scopes = [root, ...descendants(root).filter(e => e.shadowRoot).map(e => e.shadowRoot)];
        if (root.shadowRoot) scopes.push(root.shadowRoot);
        return scopes.flatMap(scope => [...scope.querySelectorAll(segment.value)]);
      }
      if (segment.kind !== 'xpath') throw new Error(`Unknown selector engine: ${segment.kind}`);
      const value = root !== document && segment.value.startsWith('/') ? `.${segment.value}` : segment.value;
      const snapshot = document.evaluate(value, root, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
      return Array.from({length: snapshot.snapshotLength}, (_, i) => snapshot.snapshotItem(i)).filter(e => e.nodeType === 1);
    }));
  }
  return roots;
}
