import { cardLanguage } from "./game-localization.js";
// Load the catalog as a script so startup only uses the existing script-src policy.
import messages from "./interface-messages.js";

const localeColumns = { en: 0, de: 1, es: 2, it: 3, pt: 4, ja: 5 };
const staticAttributes = ["aria-label", "alt", "title", "placeholder", "label", "data-page-title"];
const bindings = new WeakMap();
const pluralRules = new Map();

export function t(source, values) {
  if (typeof source === "function") return source();
  const message = Object.hasOwn(messages, source)
    ? messages[source][localeColumns[cardLanguage()]] ?? source
    : source;
  return values ? message.replace(/\{([a-zA-Z]+)\}/g, (match, key) => {
    if (!Object.hasOwn(values, key)) return match;
    const value = values[key];
    return String(typeof value === "function" ? value() : value);
  }) : message;
}

export function countText(count, singular, plural, formattedCount) {
  const language = cardLanguage();
  if (!pluralRules.has(language)) pluralRules.set(language, new Intl.PluralRules(language));
  return t(pluralRules.get(language).select(count) === "one" ? singular : plural, { count: formattedCount });
}

function bind(node, property, source, values, read, write) {
  if (!node) return;
  const value = t(source, values);
  write(value);
  let targets = bindings.get(node);
  if (!targets) bindings.set(node, targets = new Map());
  targets.set(property, { source, values, value, read, write });
}

export function setUiText(node, source, values) {
  bind(node, "textContent", source, values, () => node.textContent, (value) => { node.textContent = value; });
}

export function setUiAttribute(node, attribute, source, values) {
  bind(node, attribute, source, values, () => node.getAttribute(attribute), (value) => node.setAttribute(attribute, value));
}

function visit(node, apply) {
  if (node.tagName === "SCRIPT" || node.tagName === "STYLE") return;
  apply(node);
  for (const child of node.childNodes) visit(child, apply);
  if (node.tagName === "TEMPLATE") visit(node.content, apply);
}

// Capture only the authored HTML. Data added later is bound explicitly by the renderers.
export function initializeInterfaceLanguage(root) {
  visit(root, (node) => {
    if (node.nodeType === 3) {
      if (bindings.get(node.parentNode)?.has("textContent")) return;
      const source = node.nodeValue.trim().replace(/\s+/g, " ");
      if (!Object.hasOwn(messages, source)) return;
      if (node.parentNode?.tagName === "OPTION" && !node.parentNode.hasAttribute("value")) {
        node.parentNode.setAttribute("value", node.parentNode.value);
      }
      const leading = /^\s*/.exec(node.nodeValue)[0];
      const trailing = /\s*$/.exec(node.nodeValue)[0];
      bind(node, "nodeValue", source, undefined, () => node.nodeValue.trim(),
        (value) => { node.nodeValue = leading + value + trailing; });
    } else if (node.nodeType === 1) {
      for (const attribute of staticAttributes) {
        const source = node.getAttribute(attribute);
        if (source && Object.hasOwn(messages, source)) setUiAttribute(node, attribute, source);
      }
    }
  });
  root.lang = cardLanguage();
}

export function refreshInterfaceLanguage(root) {
  root.lang = cardLanguage();
  visit(root, (node) => {
    for (const entry of bindings.get(node)?.values() ?? []) {
      // The renderer may have replaced a placeholder with collection or release data.
      if (entry.read() !== entry.value) continue;
      const next = t(entry.source, entry.values);
      if (next !== entry.value) entry.write(entry.value = next);
    }
  });
}
