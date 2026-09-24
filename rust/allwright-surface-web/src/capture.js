// Invoked in the selected browsing context through WebDriver BiDi.
(kind, element, attributeName) => {
  const result = { value: null, checked: null, selected_options: [], bounding_box: null };
  if (kind === 'url') { result.value = location.href; return result; }
  if (!element) throw new Error('No element matches capture selector');
  switch (kind) {
    case 'input_value':
      if (!['input', 'textarea', 'select'].includes(element.localName))
        throw new Error('inputValue requires an input, textarea, or select');
      result.value = element.value;
      break;
    case 'selected_options': {
      if (element.localName === 'select') {
        result.selected_options = Array.from(element.selectedOptions, o => ({ value: o.value, label: o.label, index: o.index }));
      } else {
        const role = element.getAttribute('role');
        if (!['combobox', 'listbox'].includes(role)) throw new Error('selectedOptions requires a select, combobox, or listbox');
        const root = element.getRootNode();
        const ids = (element.getAttribute('aria-controls') || element.getAttribute('aria-owns') || '').split(/\s+/).filter(Boolean);
        const containers = [element, ...ids.map(id => root.getElementById?.(id)).filter(Boolean)];
        const options = [...new Set(containers.flatMap(c => Array.from(c.querySelectorAll('[role="option"]'))))];
        result.selected_options = options.flatMap((o, index) => o.getAttribute('aria-selected') === 'true'
          ? [{ value: o.getAttribute('value') ?? o.textContent ?? '', label: o.getAttribute('aria-label') ?? o.textContent ?? '', index }]
          : []);
      }
      break;
    }
    case 'selected_text':
      if (!['input', 'textarea'].includes(element.localName)) throw new Error('selectedText requires an input or textarea');
      if (element.selectionStart !== null && element.selectionEnd !== null)
        result.value = element.value.slice(element.selectionStart, element.selectionEnd);
      break;
    case 'checked':
      if (element.localName === 'input' && ['checkbox', 'radio'].includes(element.type)) result.checked = element.checked;
      else if (['checkbox', 'radio', 'switch', 'menuitemcheckbox', 'menuitemradio'].includes(element.getAttribute('role'))) {
        const checked = element.getAttribute('aria-checked');
        if (!['true', 'false', 'mixed'].includes(checked)) throw new Error('Checked control has no valid aria-checked state');
        result.checked = checked === 'true';
      } else throw new Error('isChecked requires a checkbox, radio, or ARIA checked control');
      break;
    case 'attribute':
      if (!attributeName) throw new Error('Attribute name must not be empty');
      result.value = element.getAttribute(attributeName);
      break;
    case 'bounding_box': {
      const style = getComputedStyle(element);
      const box = element.getBoundingClientRect();
      if (style.visibility !== 'hidden' && style.visibility !== 'collapse' && element.getClientRects().length && box.width > 0 && box.height > 0)
        result.bounding_box = { x: box.x, y: box.y, width: box.width, height: box.height };
      break;
    }
    default: throw new Error('Unknown capture kind: ' + kind);
  }
  return result;
}
