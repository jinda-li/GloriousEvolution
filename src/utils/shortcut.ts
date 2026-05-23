const MODIFIER_CODES = new Set([
  "AltLeft",
  "AltRight",
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

const NAMED_KEYS: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Escape: "Escape",
  ArrowUp: "ArrowUp",
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  Comma: "Comma",
  Period: "Period",
  Semicolon: "Semicolon",
  Quote: "Quote",
  BracketLeft: "BracketLeft",
  BracketRight: "BracketRight",
  Backslash: "Backslash",
  Slash: "Slash",
  Minus: "Minus",
  Equal: "Equal",
  Backquote: "Backquote",
};

function codeToKey(code: string): string | null {
  if (MODIFIER_CODES.has(code)) {
    return null;
  }

  if (code.startsWith("Key")) {
    return code.slice(3);
  }

  if (code.startsWith("Digit")) {
    return code.slice(5);
  }

  if (code.startsWith("F") && code.length <= 3) {
    return code;
  }

  return NAMED_KEYS[code] ?? null;
}

export function keyboardEventToShortcut(event: KeyboardEvent): string | null {
  const key = codeToKey(event.code);
  if (!key) {
    return null;
  }

  const modifiers: string[] = [];
  if (event.ctrlKey) {
    modifiers.push("Control");
  }
  if (event.altKey) {
    modifiers.push("Alt");
  }
  if (event.shiftKey) {
    modifiers.push("Shift");
  }
  if (event.metaKey) {
    modifiers.push("Super");
  }

  const needsModifier = /^[A-Z0-9]$/.test(key);
  if (needsModifier && modifiers.length === 0) {
    return null;
  }

  return [...modifiers, key].join("+");
}

export function keyboardEventPreview(event: KeyboardEvent): string | null {
  if (MODIFIER_CODES.has(event.code)) {
    const modifiers: string[] = [];
    if (event.ctrlKey) {
      modifiers.push("Control");
    }
    if (event.altKey) {
      modifiers.push("Alt");
    }
    if (event.shiftKey) {
      modifiers.push("Shift");
    }
    if (event.metaKey) {
      modifiers.push("Super");
    }

    return modifiers.length > 0 ? `${modifiers.join("+")}+…` : null;
  }

  return keyboardEventToShortcut(event);
}
