// Allwright-owned DOM accessibility semantics. Specification references and known
// limits are documented in ACCESSIBILITY.md. No external implementation is bundled.
const allwrightA11y = (() => {
  const words = text => new Set(text.split(/\s+/));
  const roles = words('alert alertdialog application article banner blockquote button caption cell checkbox code columnheader combobox complementary contentinfo definition deletion dialog directory document emphasis feed figure form generic grid gridcell group heading img insertion link list listbox listitem log main mark marquee math menu menubar menuitem menuitemcheckbox menuitemradio meter navigation none note option paragraph presentation progressbar radio radiogroup region row rowgroup rowheader scrollbar search searchbox separator slider spinbutton status strong subscript suggestion superscript switch tab table tablist tabpanel term textbox time timer toolbar tooltip tree treegrid treeitem');
  const contentNames = words('button cell checkbox columnheader gridcell heading link menuitem menuitemcheckbox menuitemradio option radio row rowheader switch tab tooltip treeitem');
  const noNames = words('caption code deletion emphasis generic insertion none paragraph presentation strong subscript superscript');
  const globalAttributes = words('atomic busy controls current describedby description details dropeffect errormessage flowto grabbed haspopup hidden invalid keyshortcuts label labelledby live owns relevant roledescription');
  const tagRoles = {
    article: 'article', aside: 'complementary', blockquote: 'blockquote', button: 'button',
    caption: 'caption', code: 'code', datalist: 'listbox', dd: 'definition', del: 'deletion',
    details: 'group', dfn: 'term', dialog: 'dialog', dt: 'term', em: 'emphasis', fieldset: 'group',
    figure: 'figure', hr: 'separator', html: 'document', ins: 'insertion', li: 'listitem',
    main: 'main', mark: 'mark', math: 'math', menu: 'list', meter: 'meter', nav: 'navigation',
    ol: 'list', optgroup: 'group', option: 'option', output: 'status', p: 'paragraph',
    progress: 'progressbar', search: 'search', strong: 'strong', sub: 'subscript',
    sup: 'superscript', svg: 'img', table: 'table', tbody: 'rowgroup', textarea: 'textbox',
    tfoot: 'rowgroup', thead: 'rowgroup', time: 'time', tr: 'row', ul: 'list',
  };
  const normalize = text => String(text ?? '').replace(/[\t\r\n\f ]+/g, ' ').trim();
  const parent = element => element.assignedSlot || element.parentElement || element.getRootNode().host;
  const styleCache = new WeakMap();
  function style(element) {
    if (!styleCache.has(element)) styleCache.set(element, element.ownerDocument.defaultView.getComputedStyle(element));
    return styleCache.get(element);
  }
  function children(element) {
    if (element.localName === 'slot') {
      const assigned = element.assignedNodes({ flatten: true });
      if (assigned.length) return assigned;
    }
    return Array.from((element.shadowRoot || element).childNodes);
  }
  function references(element, attribute) {
    const root = element.getRootNode();
    return [...new Set((element.getAttribute(attribute) || '').split(/\s+/).filter(Boolean))]
      .map(id => root.getElementById?.(id)).filter(Boolean);
  }
  function explicitRole(element) {
    return (element.getAttribute('role') || '').split(/\s+/).find(role => roles.has(role)) || null;
  }
  function presentationConflict(element) {
    return element.tabIndex >= 0 || element.hasAttribute('tabindex') ||
      Array.from(element.attributes).some(attribute => attribute.name.startsWith('aria-') && globalAttributes.has(attribute.name.slice(5)));
  }
  function nearest(element, predicate) {
    for (let ancestor = parent(element); ancestor; ancestor = parent(ancestor)) {
      if (predicate(ancestor)) return ancestor;
    }
    return null;
  }
  function implicitRole(element) {
    const tag = element.localName;
    if (/^h[1-6]$/.test(tag)) return 'heading';
    if (['a', 'area'].includes(tag)) return element.hasAttribute('href') ? 'link' : null;
    if (tag === 'input') {
      const type = element.type;
      if (type === 'hidden') return null;
      if (['button', 'submit', 'reset', 'image', 'file'].includes(type)) return 'button';
      if (['checkbox', 'radio'].includes(type)) return type;
      if (type === 'range') return 'slider';
      if (type === 'number') return 'spinbutton';
      if (element.list && ['text', 'search', 'email', 'url', 'tel'].includes(type)) return 'combobox';
      return type === 'search' ? 'searchbox' : 'textbox';
    }
    if (tag === 'select') return element.multiple || element.size > 1 ? 'listbox' : 'combobox';
    if (tag === 'img') return element.getAttribute('alt') === '' && !element.getAttribute('title') && !presentationConflict(element) ? 'presentation' : 'img';
    if (['form', 'section'].includes(tag)) {
      return normalize(element.getAttribute('aria-label')) || references(element, 'aria-labelledby').length ? (tag === 'form' ? 'form' : 'region') : null;
    }
    if (['header', 'footer'].includes(tag)) {
      const scoped = nearest(element, ancestor => ['article', 'aside', 'main', 'nav', 'section'].includes(ancestor.localName) ||
        ['article', 'complementary', 'main', 'navigation', 'region'].includes(explicitRole(ancestor)));
      return scoped ? null : tag === 'header' ? 'banner' : 'contentinfo';
    }
    if (tag === 'summary') return element.parentElement?.localName === 'details' &&
      Array.from(element.parentElement.children).find(child => child.localName === 'summary') === element ? 'button' : null;
    if (tag === 'th') return ['row', 'rowgroup'].includes(element.scope) ? 'rowheader' : 'columnheader';
    if (tag === 'td') {
      const table = nearest(element, ancestor => ancestor.localName === 'table');
      return table && ['grid', 'treegrid'].includes(explicitRole(table)) ? 'gridcell' : 'cell';
    }
    if (['iframe', 'frame'].includes(tag)) return 'iframe';
    return tagRoles[tag] || null;
  }
  function getRole(element) {
    const explicit = explicitRole(element);
    if (explicit && (!['none', 'presentation'].includes(explicit) || !presentationConflict(element))) return explicit;
    // HTML's structural children inherit presentation from lists/tables unless
    // they explicitly establish semantics or are themselves interactive.
    if (!explicit && !presentationConflict(element)) {
      const chains = { li: ['ul', 'ol', 'menu'], tr: ['table', 'tbody', 'thead', 'tfoot'],
        td: ['tr'], th: ['tr'], tbody: ['table'], thead: ['table'], tfoot: ['table'] };
      let current = element;
      while (chains[current.localName]?.includes(current.parentElement?.localName)) {
        current = current.parentElement;
        const role = explicitRole(current);
        if (['none', 'presentation'].includes(role) && !presentationConflict(current)) return 'presentation';
        if (role) break;
      }
    }
    return implicitRole(element);
  }
  const excluded = new WeakMap();
  function hiddenSubtree(element) {
    if (!element) return false;
    if (excluded.has(element)) return excluded.get(element);
    const ancestor = parent(element);
    const hidden = ['script', 'style', 'template', 'noscript', 'head'].includes(element.localName) ||
      element.getAttribute('aria-hidden')?.toLowerCase() === 'true' || element.inert ||
      style(element).display === 'none' || style(element).contentVisibility === 'hidden' ||
      (element.localName === 'input' && element.type === 'hidden') ||
      (element.parentElement?.shadowRoot && !element.assignedSlot) ||
      (ancestor?.localName === 'details' && !ancestor.open &&
        element !== Array.from(ancestor.children).find(child => child.localName === 'summary')) ||
      hiddenSubtree(ancestor);
    excluded.set(element, !!hidden);
    return !!hidden;
  }
  function visible(element) {
    // Options remain accessible within native selects, even when their popup is closed.
    return !hiddenSubtree(element) && (element.localName === 'option' || !['hidden', 'collapse'].includes(style(element).visibility));
  }
  function cssText(element, pseudo) {
    const computed = element.ownerDocument.defaultView.getComputedStyle(element, pseudo);
    if (computed.display === 'none' || computed.visibility === 'hidden') return '';
    const content = computed.content || '';
    let text = '';
    // Computed CSS strings can contain hex escapes, multiple quoted fragments,
    // and attr() values. Ignore images/counters rather than inventing text.
    let depth = 0;
    for (let index = 0; index < content.length; index++) {
      const token = content[index];
      if (token === '(') { depth++; continue; }
      if (token === ')') { depth = Math.max(0, depth - 1); continue; }
      if (token === '/' && depth === 0) { text = ''; continue; }
      if (token !== '"' && token !== "'") continue;
      let raw = '';
      while (++index < content.length && content[index] !== token) {
        raw += content[index];
        if (content[index] === '\\' && index + 1 < content.length) raw += content[++index];
      }
      if (depth !== 0) continue;
      text += raw.replace(/\\([0-9a-fA-F]{1,6})\s?|\\([^\r\n])/g, (_, hex, character) => {
        if (!hex) return character;
        const code = parseInt(hex, 16);
        return code > 0 && code <= 0x10ffff && !(code >= 0xd800 && code <= 0xdfff) ? String.fromCodePoint(code) : '\ufffd';
      });
    }
    return text;
  }
  function computeAccessibleName(element) {
    if (noNames.has(getRole(element)) || !visible(element)) return '';
    return normalize(textAlternative(element, { path: new Set(), referenced: false, content: false, includeHidden: false }));
  }
  function textAlternative(node, context) {
    if (node.nodeType === 3) return node.nodeValue || '';
    if (node.nodeType !== 1 || context.path.has(node)) return '';
    if (!context.includeHidden && hiddenSubtree(node)) return '';
    const visibleHere = context.includeHidden || visible(node);
    const next = { ...context, path: new Set(context.path).add(node) };
    // References are followed once; an IDREF chain cannot recurse indefinitely.
    if (visibleHere && !context.referenced) {
      const labels = references(node, 'aria-labelledby');
      if (labels.length) return labels.map(label => textAlternative(label, {
        ...next, referenced: true, content: true, includeHidden: !visible(label),
        // A self reference may use aria-label/native content, but never follows itself again.
        path: label === node ? context.path : next.path,
      })).join(' ');
    }
    const label = normalize(node.getAttribute('aria-label'));
    if (visibleHere && label) return label;
    const tag = node.localName;
    if (visibleHere && ['img', 'area'].includes(tag) && node.hasAttribute('alt')) return node.getAttribute('alt');
    if (visibleHere && context.content) {
      if (tag === 'input' && !['checkbox', 'radio', 'password', 'file'].includes(node.type)) return node.value;
      if (tag === 'textarea') return node.value;
      if (tag === 'select') return Array.from(node.selectedOptions).map(option => textAlternative(option, { ...next, content: true })).join(' ');
      if (['slider', 'spinbutton', 'scrollbar', 'progressbar'].includes(getRole(node))) return node.getAttribute('aria-valuetext') || node.getAttribute('aria-valuenow') || '';
    }
    if (visibleHere && node.labels?.length) {
      return Array.from(node.labels).map(label => textAlternative(label, { ...next, content: true, includeHidden: !visible(label) })).join(' ');
    }
    if (visibleHere && tag === 'input') {
      if (node.type === 'image') return node.getAttribute('alt') || node.getAttribute('title') || 'Submit';
      if (['button', 'submit', 'reset'].includes(node.type)) return node.value || (node.type === 'submit' ? 'Submit' : node.type === 'reset' ? 'Reset' : '');
    }
    const nativeLabelTag = { fieldset: 'legend', table: 'caption', figure: 'figcaption', svg: 'title', optgroup: null }[tag];
    if (visibleHere && nativeLabelTag) {
      const nativeLabel = Array.from(node.children).find(child => child.localName === nativeLabelTag);
      if (nativeLabel) return textAlternative(nativeLabel, { ...next, content: true });
    }
    if (visibleHere && tag === 'optgroup' && node.hasAttribute('label')) return node.getAttribute('label');
    if (context.content || contentNames.has(getRole(node))) {
      let result = visibleHere ? cssText(node, '::before') : '';
      const seen = new Set();
      for (const child of [...children(node), ...references(node, 'aria-owns')]) {
        if (seen.has(child) || (child.nodeType === 3 && !visibleHere)) continue;
        seen.add(child);
        const fragment = textAlternative(child, { ...next, content: true });
        const separate = child.nodeType === 1 && (child.localName === 'br' || style(child).display !== 'inline');
        result += separate ? ` ${fragment} ` : fragment;
      }
      if (visibleHere) result += cssText(node, '::after');
      if (normalize(result)) return result;
    }
    if (!visibleHere) return '';
    return node.getAttribute('title') || (['input', 'textarea'].includes(tag) ? node.getAttribute('placeholder') || '' : '');
  }
  function computeAccessibleDescription(element) {
    const descriptions = references(element, 'aria-describedby');
    if (descriptions.length) return normalize(descriptions.map(node => textAlternative(node, {
      path: new Set([element]), referenced: true, content: true, includeHidden: !visible(node),
    })).join(' '));
    if (element.hasAttribute('aria-description')) return normalize(element.getAttribute('aria-description'));
    const desc = element.localName === 'svg' && Array.from(element.children).find(child => child.localName === 'desc');
    if (desc) return normalize(textAlternative(desc, { path: new Set(), referenced: true, content: true, includeHidden: true }));
    const title = normalize(element.getAttribute('title'));
    return title !== computeAccessibleName(element) ? title : '';
  }
  return { getRole, computeAccessibleName, computeAccessibleDescription, hiddenSubtree, visible, cssText, children, normalize };
})();
