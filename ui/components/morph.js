/**
 * Patches `target`'s children to match `html` in place. Live surfaces re-render every
 * second; replacing nodes wholesale would drop hover state and swallow clicks that
 * straddle an update, so matching nodes are updated rather than recreated.
 */
export function morph(target, html) {
  const template = document.createElement("template");
  template.innerHTML = html.trim();
  patchChildren(target, template.content);
}

function patchChildren(current, next) {
  const currentNodes = [...current.childNodes].filter(isMeaningful);
  const nextNodes = [...next.childNodes].filter(isMeaningful);
  [...current.childNodes].forEach((node) => {
    if (!isMeaningful(node)) node.remove();
  });

  nextNodes.forEach((nextNode, index) => {
    const existing = currentNodes[index];
    if (!existing) {
      current.appendChild(nextNode);
    } else if (!sameKind(existing, nextNode)) {
      current.replaceChild(nextNode, existing);
    } else if (existing.nodeType === Node.TEXT_NODE) {
      if (existing.nodeValue !== nextNode.nodeValue) existing.nodeValue = nextNode.nodeValue;
    } else {
      patchAttributes(existing, nextNode);
      patchChildren(existing, nextNode);
    }
  });
  currentNodes.slice(nextNodes.length).forEach((node) => node.remove());
}

function patchAttributes(element, next) {
  for (const { name } of [...element.attributes]) {
    if (!next.hasAttribute(name)) element.removeAttribute(name);
  }
  for (const { name, value } of [...next.attributes]) {
    if (element.getAttribute(name) !== value) element.setAttribute(name, value);
  }
}

function sameKind(a, b) {
  return a.nodeType === b.nodeType && a.nodeName === b.nodeName;
}

function isMeaningful(node) {
  return node.nodeType === Node.ELEMENT_NODE || (node.nodeType === Node.TEXT_NODE && node.nodeValue.trim() !== "");
}
